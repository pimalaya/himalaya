//! Deserialized config module.
//!
//! This module contains the raw deserialized representation of the
//! user configuration file.

use anyhow::{anyhow, Context, Result};
use dirs::{config_dir, home_dir};
use log::{debug, trace};
use pimalaya_email::{AccountConfig, BackendConfig, EmailHooks, EmailTextPlainFormat};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use toml;

use crate::{
    account::DeserializedAccountConfig,
    config::{prelude::*, wizard::wizard},
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
    pub email_reading_headers: Option<Vec<String>>,
    #[serde(default, with = "EmailTextPlainFormatDef")]
    pub email_reading_format: EmailTextPlainFormat,
    pub email_reading_verify_cmd: Option<String>,
    pub email_reading_decrypt_cmd: Option<String>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_writing_sign_cmd: Option<String>,
    pub email_writing_encrypt_cmd: Option<String>,
    #[serde(
        default,
        with = "EmailHooksDef",
        skip_serializing_if = "EmailHooks::is_empty"
    )]
    pub email_hooks: EmailHooks,

    #[serde(flatten)]
    pub accounts: HashMap<String, DeserializedAccountConfig>,
}

impl DeserializedConfig {
    /// Tries to create a config from an optional path.
    pub fn from_opt_path(path: Option<&str>) -> Result<Self> {
        debug!("path: {:?}", path);

        let config: Self = match path.map(|s| s.into()).or_else(Self::path) {
            Some(path) => {
                let content = fs::read_to_string(path).context("cannot read config file")?;
                toml::from_str(&content).context("cannot parse config file")?
            }
            None => wizard()?,
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

    pub fn to_configs(&self, account_name: Option<&str>) -> Result<(AccountConfig, BackendConfig)> {
        let (account_name, deserialized_account_config) = match account_name {
            Some("default") | Some("") | None => self
                .accounts
                .iter()
                .find_map(|(name, account)| {
                    if account.is_default() {
                        Some((name.clone(), account))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => self
                .accounts
                .get(name)
                .map(|account| (name.to_string(), account))
                .ok_or_else(|| anyhow!(format!("cannot find account {}", name))),
        }?;

        let (account_config, backend_config) =
            deserialized_account_config.to_configs(account_name, self);

        Ok((account_config, backend_config))
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_email::{EmailSender, MaildirConfig, SendmailConfig, SmtpConfig};

    #[cfg(feature = "notmuch-backend")]
    use pimalaya_email::NotmuchConfig;
    #[cfg(feature = "imap-backend")]
    use pimalaya_email::{ImapAuthConfig, ImapConfig};

    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::account::DeserializedBaseAccountConfig;

    #[cfg(feature = "imap-backend")]
    use crate::account::DeserializedImapAccountConfig;
    use crate::account::DeserializedMaildirAccountConfig;
    #[cfg(feature = "notmuch-backend")]
    use crate::account::DeserializedNotmuchAccountConfig;

    use super::*;

    fn make_config(config: &str) -> Result<DeserializedConfig> {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", config).unwrap();
        DeserializedConfig::from_opt_path(file.into_temp_path().to_str())
    }

    #[test]
    fn empty_config() {
        let config = make_config("");

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "config file must contain at least one account"
        );
    }

    #[test]
    fn account_missing_backend_field() {
        let config = make_config("[account]");

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `backend`"));
    }

    #[test]
    fn account_invalid_backend_field() {
        let config = make_config(
            "[account]
            backend = \"bad\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("unknown variant `bad`"));
    }

    #[test]
    fn account_missing_email_field() {
        let config = make_config(
            "[account]
            backend = \"none\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `email`"));
    }

    #[test]
    fn imap_account_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-host`"));
    }

    #[test]
    fn account_backend_imap_missing_port_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-port`"));
    }

    #[test]
    fn account_backend_imap_missing_login_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-login`"));
    }

    #[test]
    fn account_backend_imap_missing_passwd_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993
            imap-login = \"login\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `imap-auth`"));
    }

    #[test]
    fn account_backend_maildir_missing_root_dir_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"maildir\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `maildir-root-dir`"));
    }

    #[cfg(feature = "notmuch-backend")]
    #[test]
    fn account_backend_notmuch_missing_db_path_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"notmuch\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `notmuch-db-path`"));
    }

    #[test]
    fn account_missing_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `sender`"));
    }

    #[test]
    fn account_invalid_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"bad\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("unknown variant `bad`, expected one of `none`, `smtp`, `sendmail`"),);
    }

    #[test]
    fn account_smtp_sender_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-host`"));
    }

    #[test]
    fn account_smtp_sender_missing_port_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-port`"));
    }

    #[test]
    fn account_smtp_sender_missing_login_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-login`"));
    }

    #[test]
    fn account_smtp_sender_missing_passwd_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25
            smtp-login = \"login\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `smtp-passwd-cmd`"));
    }

    #[test]
    fn account_sendmail_sender_missing_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"sendmail\"",
        );

        assert!(config
            .unwrap_err()
            .root_cause()
            .to_string()
            .contains("missing field `sendmail-cmd`"));
    }

    #[test]
    fn account_smtp_sender_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"smtp\"
            smtp-host = \"localhost\"
            smtp-port = 25
            smtp-login = \"login\"
            smtp-passwd-cmd = \"echo password\"",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::None(DeserializedBaseAccountConfig {
                        email: "test@localhost".into(),
                        email_sender: EmailSender::Smtp(SmtpConfig {
                            host: "localhost".into(),
                            port: 25,
                            login: "login".into(),
                            passwd_cmd: "echo password".into(),
                            ..SmtpConfig::default()
                        }),
                        ..DeserializedBaseAccountConfig::default()
                    })
                )]),
                ..DeserializedConfig::default()
            }
        );
    }

    #[test]
    fn account_sendmail_sender_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"sendmail\"
            sendmail-cmd = \"echo send\"",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::None(DeserializedBaseAccountConfig {
                        email: "test@localhost".into(),
                        email_sender: EmailSender::Sendmail(SendmailConfig {
                            cmd: "echo send".into(),
                        }),
                        ..DeserializedBaseAccountConfig::default()
                    })
                )]),
                ..DeserializedConfig::default()
            }
        );
    }

    #[test]
    fn account_backend_imap_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"
            imap-host = \"localhost\"
            imap-port = 993
            imap-login = \"login\"
            imap-auth = { raw-passwd = \"password\" }",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::Imap(DeserializedImapAccountConfig {
                        base: DeserializedBaseAccountConfig {
                            email: "test@localhost".into(),
                            ..DeserializedBaseAccountConfig::default()
                        },
                        backend: ImapConfig {
                            host: "localhost".into(),
                            port: 993,
                            login: "login".into(),
                            auth: ImapAuthConfig::RawPasswd("password".into()),
                            ..ImapConfig::default()
                        }
                    })
                )]),
                ..DeserializedConfig::default()
            }
        );
    }
    #[test]
    fn account_backend_maildir_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"maildir\"
            maildir-root-dir = \"/tmp/maildir\"",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::Maildir(DeserializedMaildirAccountConfig {
                        base: DeserializedBaseAccountConfig {
                            email: "test@localhost".into(),
                            ..DeserializedBaseAccountConfig::default()
                        },
                        backend: MaildirConfig {
                            root_dir: "/tmp/maildir".into(),
                        }
                    })
                )]),
                ..DeserializedConfig::default()
            }
        );
    }

    #[cfg(feature = "notmuch-backend")]
    #[test]
    fn account_backend_notmuch_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"notmuch\"
            notmuch-db-path = \"/tmp/notmuch.db\"",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::Notmuch(DeserializedNotmuchAccountConfig {
                        base: DeserializedBaseAccountConfig {
                            email: "test@localhost".into(),
                            ..DeserializedBaseAccountConfig::default()
                        },
                        backend: NotmuchConfig {
                            db_path: "/tmp/notmuch.db".into(),
                        }
                    })
                )]),
                ..DeserializedConfig::default()
            }
        );
    }
}
