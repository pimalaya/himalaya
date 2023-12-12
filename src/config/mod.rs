pub mod args;
pub mod wizard;

use anyhow::{anyhow, Context, Result};
use dialoguer::Confirm;
use dirs::{config_dir, home_dir};
use email::{
    account::config::AccountConfig, config::Config, envelope::config::EnvelopeConfig,
    folder::config::FolderConfig, message::config::MessageConfig,
};
use serde::{Deserialize, Serialize};
use shellexpand_utils::{canonicalize, expand};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process,
};
use toml;

use crate::{account::config::TomlAccountConfig, backend::BackendKind, wizard_prompt, wizard_warn};

/// Represents the user config file.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlConfig {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    #[serde(flatten)]
    pub accounts: HashMap<String, TomlAccountConfig>,
}

impl TomlConfig {
    /// Read and parse the TOML configuration at the given path.
    ///
    /// Returns an error if the configuration file cannot be read or
    /// if its content cannot be parsed.
    fn from_path(path: &Path) -> Result<Self> {
        let content =
            fs::read_to_string(path).context(format!("cannot read config file at {path:?}"))?;
        toml::from_str(&content).context(format!("cannot parse config file at {path:?}"))
    }

    /// Create and save a TOML configuration using the wizard.
    ///
    /// If the user accepts the confirmation, the wizard starts and
    /// help him to create his configuration file. Otherwise the
    /// program stops.
    ///
    /// NOTE: the wizard can only be used with interactive shells.
    async fn from_wizard(path: PathBuf) -> Result<Self> {
        wizard_warn!("Cannot find existing configuration at {path:?}.");

        let confirm = Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to create one with the wizard?"
            ))
            .default(true)
            .interact_opt()?
            .unwrap_or_default();

        if !confirm {
            process::exit(0);
        }

        wizard::configure(path).await
    }

    /// Read and parse the TOML configuration from default paths.
    pub async fn from_default_paths() -> Result<Self> {
        match Self::first_valid_default_path() {
            Some(path) => Self::from_path(&path),
            None => Self::from_wizard(Self::default_path()?).await,
        }
    }

    /// Read and parse the TOML configuration at the optional given
    /// path.
    ///
    /// If the given path exists, then read and parse the TOML
    /// configuration from it.
    ///
    /// If the given path does not exist, then create it using the
    /// wizard.
    ///
    /// If no path is given, then either read and parse the TOML
    /// configuration at the first valid default path, otherwise
    /// create it using the wizard.  wizard.
    pub async fn from_some_path_or_default(path: Option<impl Into<PathBuf>>) -> Result<Self> {
        match path.map(Into::into) {
            Some(ref path) if path.exists() => Self::from_path(path),
            Some(path) => Self::from_wizard(path).await,
            None => Self::from_default_paths().await,
        }
    }

    /// Get the default configuration path.
    ///
    /// Returns an error if the XDG configuration directory cannot be
    /// found.
    pub fn default_path() -> Result<PathBuf> {
        Ok(config_dir()
            .ok_or(anyhow!("cannot get XDG config directory"))?
            .join("himalaya")
            .join("config.toml"))
    }

    /// Get the first default configuration path that points to a
    /// valid file.
    ///
    /// Tries paths in this order:
    ///
    /// - `$XDG_CONFIG_DIR/himalaya/config.toml` (or equivalent to
    ///   `$XDG_CONFIG_DIR` in other OSes.)
    /// - `$HOME/.config/himalaya/config.toml`
    /// - `$HOME/.himalayarc`
    pub fn first_valid_default_path() -> Option<PathBuf> {
        Self::default_path()
            .ok()
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(".config").join("himalaya").join("config.toml")))
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(".himalayarc")))
            .filter(|p| p.exists())
    }

    pub fn into_toml_account_config(
        &self,
        account_name: Option<&str>,
    ) -> Result<(String, TomlAccountConfig)> {
        let (account_name, mut toml_account_config) = match account_name {
            Some("default") | Some("") | None => self
                .accounts
                .iter()
                .find_map(|(name, account)| {
                    account
                        .default
                        .filter(|default| *default == true)
                        .map(|_| (name.to_owned(), account.clone()))
                })
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => self
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account.clone()))
                .ok_or_else(|| anyhow!("cannot find account {name}")),
        }?;

        #[cfg(feature = "imap")]
        if let Some(imap_config) = toml_account_config.imap.as_mut() {
            imap_config
                .auth
                .replace_undefined_keyring_entries(&account_name);
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp_config) = toml_account_config.smtp.as_mut() {
            smtp_config
                .auth
                .replace_undefined_keyring_entries(&account_name);
        }

        Ok((account_name, toml_account_config))
    }

    /// Build account configurations from a given account name.
    pub fn into_account_configs(
        self,
        account_name: Option<&str>,
        disable_cache: bool,
    ) -> Result<(TomlAccountConfig, AccountConfig)> {
        let (account_name, mut toml_account_config) =
            self.into_toml_account_config(account_name)?;

        if let Some(true) = toml_account_config.sync.as_ref().and_then(|c| c.enable) {
            if !disable_cache {
                toml_account_config.backend = Some(BackendKind::MaildirForSync);
            }
        }

        let config = Config {
            display_name: self.display_name,
            signature: self.signature,
            signature_delim: self.signature_delim,
            downloads_dir: self.downloads_dir,

            accounts: HashMap::from_iter(self.accounts.clone().into_iter().map(
                |(name, config)| {
                    (
                        name.clone(),
                        AccountConfig {
                            name,
                            email: config.email,
                            display_name: config.display_name,
                            signature: config.signature,
                            signature_delim: config.signature_delim,
                            downloads_dir: config.downloads_dir,

                            folder: config.folder.map(|c| FolderConfig {
                                aliases: c.alias,
                                list: c.list.map(|c| c.remote),
                            }),
                            envelope: config.envelope.map(|c| EnvelopeConfig {
                                list: c.list.map(|c| c.remote),
                            }),
                            message: config.message.map(|c| MessageConfig {
                                read: c.read.map(|c| c.remote),
                                write: c.write.map(|c| c.remote),
                                send: c.send.map(|c| c.remote),
                            }),
                            sync: config.sync,
                            #[cfg(feature = "pgp")]
                            pgp: config.pgp,
                        },
                    )
                },
            )),
        };

        let account_config = config.account(&account_name)?;

        Ok((toml_account_config, account_config))
    }
}

/// Parse a configuration file path as [`PathBuf`].
///
/// The path is shell-expanded then canonicalized (if applicable).
pub fn path_parser(path: &str) -> Result<PathBuf, String> {
    expand::try_path(path)
        .map(canonicalize::path)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use email::{
        account::config::passwd::PasswdConfig, maildir::config::MaildirConfig,
        sendmail::config::SendmailConfig,
    };
    use secret::Secret;

    #[cfg(feature = "notmuch")]
    use email::backend::NotmuchConfig;
    #[cfg(feature = "imap")]
    use email::imap::config::{ImapAuthConfig, ImapConfig};
    #[cfg(feature = "smtp")]
    use email::smtp::config::{SmtpAuthConfig, SmtpConfig};

    use std::io::Write;
    use tempfile::NamedTempFile;

    use super::*;

    async fn make_config(config: &str) -> Result<TomlConfig> {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", config).unwrap();
        TomlConfig::from_some_path_or_default(file.into_temp_path().to_str()).await
    }

    #[tokio::test]
    async fn empty_config() {
        let config = make_config("").await;

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "config file must contain at least one account"
        );
    }

    #[tokio::test]
    async fn account_missing_email_field() {
        let config = make_config("[account]").await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `email`"));
    }

    #[tokio::test]
    async fn account_missing_backend_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `backend`"));
    }

    #[tokio::test]
    async fn account_invalid_backend_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"bad\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("unknown variant `bad`"));
    }

    #[tokio::test]
    async fn imap_account_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-host`"));
    }

    #[tokio::test]
    async fn account_backend_imap_missing_port_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-port`"));
    }

    #[tokio::test]
    async fn account_backend_imap_missing_login_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-login`"));
    }

    #[tokio::test]
    async fn account_backend_imap_missing_passwd_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993
            imap-login = \"login\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-auth`"));
    }

    #[tokio::test]
    async fn account_backend_maildir_missing_root_dir_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"maildir\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `maildir-root-dir`"));
    }

    #[cfg(feature = "notmuch")]
    #[tokio::test]
    async fn account_backend_notmuch_missing_db_path_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"notmuch\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `notmuch-db-path`"));
    }

    #[tokio::test]
    async fn account_missing_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `sender`"));
    }

    #[tokio::test]
    async fn account_invalid_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"bad\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("unknown variant `bad`, expected one of `none`, `smtp`, `sendmail`"),);
    }

    #[tokio::test]
    async fn account_smtp_sender_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-host`"));
    }

    #[tokio::test]
    async fn account_smtp_sender_missing_port_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-port`"));
    }

    #[tokio::test]
    async fn account_smtp_sender_missing_login_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-login`"));
    }

    #[tokio::test]
    async fn account_smtp_sender_missing_auth_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25
            smtp-login = \"login\"",
        )
        .await;

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-auth`"));
    }

    #[tokio::test]
    async fn account_sendmail_sender_missing_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"sendmail\"",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        sender: SenderConfig::Sendmail(SendmailConfig {
                            cmd: "/usr/sbin/sendmail".into()
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        )
    }

    #[cfg(feature = "smtp")]
    #[tokio::test]
    async fn account_smtp_sender_minimum_config() {
        use email::sender::SenderConfig;

        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25
            smtp-login = \"login\"
            smtp-auth = \"passwd\"
            smtp-passwd = { cmd  = \"echo password\" }",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        sender: SenderConfig::Smtp(SmtpConfig {
                            host: "localhost".into(),
                            port: 25,
                            login: "login".into(),
                            auth: SmtpAuthConfig::Passwd(PasswdConfig {
                                passwd: Secret::new_cmd(String::from("echo password"))
                            }),
                            ..SmtpConfig::default()
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        )
    }

    #[tokio::test]
    async fn account_sendmail_sender_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"sendmail\"
            sendmail-cmd = \"echo send\"",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        sender: SenderConfig::Sendmail(SendmailConfig {
                            cmd: Cmd::from("echo send")
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        )
    }

    #[tokio::test]
    async fn account_backend_imap_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993
            imap-login = \"login\"
            imap-auth = \"passwd\"
            imap-passwd = { cmd = \"echo password\" }",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        backend: BackendConfig::Imap(ImapConfig {
                            host: "localhost".into(),
                            port: 993,
                            login: "login".into(),
                            auth: ImapAuthConfig::Passwd(PasswdConfig {
                                passwd: Secret::new_cmd(String::from("echo password"))
                            }),
                            ..ImapConfig::default()
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        )
    }

    #[tokio::test]
    async fn account_backend_maildir_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"maildir\"
            maildir-root-dir = \"/tmp/maildir\"",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        backend: BackendConfig::Maildir(MaildirConfig {
                            root_dir: "/tmp/maildir".into(),
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        )
    }

    #[cfg(feature = "notmuch")]
    #[tokio::test]
    async fn account_backend_notmuch_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"notmuch\"
            notmuch-db-path = \"/tmp/notmuch.db\"",
        )
        .await;

        assert_eq!(
            config.unwrap(),
            TomlConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    TomlAccountConfig {
                        email: "test@localhost".into(),
                        backend: BackendConfig::Notmuch(NotmuchConfig {
                            db_path: "/tmp/notmuch.db".into(),
                        }),
                        ..TomlAccountConfig::default()
                    }
                )]),
                ..TomlConfig::default()
            }
        );
    }
}
