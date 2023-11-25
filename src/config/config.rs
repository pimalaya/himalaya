//! Deserialized config module.
//!
//! This module contains the raw deserialized representation of the
//! user configuration file.

use anyhow::{anyhow, Context, Result};
use dialoguer::Confirm;
use dirs::{config_dir, home_dir};
use email::email::{EmailHooks, EmailTextPlainFormat};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, process::exit};
use toml;

use crate::{
    account::DeserializedAccountConfig,
    config::{prelude::*, wizard},
    wizard_prompt, wizard_warn,
};

/// Represents the user config file.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeserializedConfig {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature_delim: Option<String>,
    pub signature: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    pub folder_listing_page_size: Option<usize>,
    pub folder_aliases: Option<HashMap<String, String>>,

    pub email_listing_page_size: Option<usize>,
    pub email_listing_datetime_fmt: Option<String>,
    pub email_listing_datetime_local_tz: Option<bool>,
    pub email_reading_headers: Option<Vec<String>>,
    #[serde(default, with = "OptionEmailTextPlainFormatDef")]
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_sending_save_copy: Option<bool>,
    #[serde(default, with = "OptionEmailHooksDef")]
    pub email_hooks: Option<EmailHooks>,

    #[serde(flatten)]
    pub accounts: HashMap<String, DeserializedAccountConfig>,
}

impl DeserializedConfig {
    /// Tries to create a config from an optional path.
    pub async fn from_opt_path(path: Option<&str>) -> Result<Self> {
        debug!("path: {:?}", path);

        let config = if let Some(path) = path.map(PathBuf::from).or_else(Self::path) {
            let content = fs::read_to_string(path).context("cannot read config file")?;
            toml::from_str(&content).context("cannot parse config file")?
        } else {
            wizard_warn!("Himalaya could not find an already existing configuration file.");

            if !Confirm::new()
                .with_prompt(wizard_prompt!(
                    "Would you like to create one with the wizard?"
                ))
                .default(true)
                .interact_opt()?
                .unwrap_or_default()
            {
                exit(0);
            }

            wizard::configure().await?
        };

        if config.accounts.is_empty() {
            return Err(anyhow!("config file must contain at least one account"));
        }

        trace!("config: {:#?}", config);
        Ok(config)
    }

    /// Tries to return a config path from a few default settings.
    ///
    /// Tries paths in this order:
    ///
    /// - `"$XDG_CONFIG_DIR/himalaya/config.toml"` (or equivalent to `$XDG_CONFIG_DIR` in  other
    ///   OSes.)
    /// - `"$HOME/.config/himalaya/config.toml"`
    /// - `"$HOME/.himalayarc"`
    ///
    /// Returns `Some(path)` if the path exists, otherwise `None`.
    pub fn path() -> Option<PathBuf> {
        config_dir()
            .map(|p| p.join("himalaya").join("config.toml"))
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(".config").join("himalaya").join("config.toml")))
            .filter(|p| p.exists())
            .or_else(|| home_dir().map(|p| p.join(".himalayarc")))
            .filter(|p| p.exists())
    }
}

#[cfg(test)]
mod tests {
    use email::{
        account::PasswdConfig,
        backend::{BackendConfig, MaildirConfig},
        sender::{SenderConfig, SendmailConfig},
    };
    use secret::Secret;

    #[cfg(feature = "notmuch-backend")]
    use email::backend::NotmuchConfig;
    #[cfg(feature = "imap-backend")]
    use email::backend::{ImapAuthConfig, ImapConfig};
    #[cfg(feature = "smtp-sender")]
    use email::sender::{SmtpAuthConfig, SmtpConfig};

    use std::io::Write;
    use tempfile::NamedTempFile;

    use super::*;

    async fn make_config(config: &str) -> Result<DeserializedConfig> {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", config).unwrap();
        DeserializedConfig::from_opt_path(file.into_temp_path().to_str()).await
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

    #[cfg(feature = "notmuch-backend")]
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
                        email: "test@localhost".into(),
                        sender: SenderConfig::Sendmail(SendmailConfig {
                            cmd: "/usr/sbin/sendmail".into()
                        }),
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
            }
        )
    }

    #[cfg(feature = "smtp-sender")]
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
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
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
                        email: "test@localhost".into(),
                        sender: SenderConfig::Sendmail(SendmailConfig {
                            cmd: Cmd::from("echo send")
                        }),
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
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
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
                        email: "test@localhost".into(),
                        backend: BackendConfig::Maildir(MaildirConfig {
                            root_dir: "/tmp/maildir".into(),
                        }),
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
            }
        )
    }

    #[cfg(feature = "notmuch-backend")]
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
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig {
                        email: "test@localhost".into(),
                        backend: BackendConfig::Notmuch(NotmuchConfig {
                            db_path: "/tmp/notmuch.db".into(),
                        }),
                        ..DeserializedAccountConfig::default()
                    }
                )]),
                ..DeserializedConfig::default()
            }
        );
    }
}
