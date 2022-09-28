// himalaya-lib, a Rust library for email management.
// Copyright (C) 2022  soywod <clement.douin@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Deserialized config module.
//!
//! This module contains the raw deserialized representation of the
//! user configuration file.

use anyhow::{anyhow, Context, Result};
use himalaya_lib::{AccountConfig, BackendConfig, EmailHooks, EmailTextPlainFormat};
use log::{debug, trace};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf};
use toml;

use crate::{account::DeserializedAccountConfig, config::prelude::*};

/// Represents the user config file.
#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
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
    #[serde(default, with = "email_text_plain_format")]
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_reading_decrypt_cmd: Option<String>,
    pub email_writing_encrypt_cmd: Option<String>,
    #[serde(default, with = "email_hooks")]
    pub email_hooks: Option<EmailHooks>,

    #[serde(flatten)]
    pub accounts: HashMap<String, DeserializedAccountConfig>,
}

impl DeserializedConfig {
    /// Tries to create a config from an optional path.
    pub fn from_opt_path(path: Option<&str>) -> Result<Self> {
        trace!(">> parse config from path");
        debug!("path: {:?}", path);

        let path = path.map(|s| s.into()).unwrap_or(Self::path()?);
        let content = fs::read_to_string(path).context("cannot read config file")?;
        let config: Self = toml::from_str(&content).context("cannot parse config file")?;

        if config.accounts.is_empty() {
            return Err(anyhow!("config file must contain at least one account"));
        }

        trace!("config: {:?}", config);
        trace!("<< parse config from path");
        Ok(config)
    }

    /// Tries to get the XDG config file path from XDG_CONFIG_HOME
    /// environment variable.
    fn path_from_xdg() -> Result<PathBuf> {
        let path = env::var("XDG_CONFIG_HOME").context("cannot read env var XDG_CONFIG_HOME")?;
        let path = PathBuf::from(path).join("himalaya").join("config.toml");
        Ok(path)
    }

    /// Tries to get the XDG config file path from HOME environment
    /// variable.
    fn path_from_xdg_alt() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path = env::var(home_var).context(format!("cannot read env var {}", &home_var))?;
        let path = PathBuf::from(path)
            .join(".config")
            .join("himalaya")
            .join("config.toml");
        Ok(path)
    }

    /// Tries to get the .himalayarc config file path from HOME
    /// environment variable.
    fn path_from_home() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path = env::var(home_var).context(format!("cannot read env var {}", &home_var))?;
        let path = PathBuf::from(path).join(".himalayarc");
        Ok(path)
    }

    /// Tries to get the config file path.
    pub fn path() -> Result<PathBuf> {
        Self::path_from_xdg()
            .or_else(|_| Self::path_from_xdg_alt())
            .or_else(|_| Self::path_from_home())
    }

    pub fn to_configs(&self, account_name: Option<&str>) -> Result<(AccountConfig, BackendConfig)> {
        let (account_config, backend_config) = match account_name {
            Some("default") | Some("") | None => self
                .accounts
                .iter()
                .find_map(|(_, account)| {
                    if account.is_default() {
                        Some(account)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => self
                .accounts
                .get(name)
                .ok_or_else(|| anyhow!(format!("cannot find account {}", name))),
        }?
        .to_configs(self);

        Ok((account_config, backend_config))
    }
}

#[cfg(test)]
mod tests {
    use himalaya_lib::{EmailSendCmd, EmailSender, SmtpConfig};

    #[cfg(feature = "imap-backend")]
    use himalaya_lib::ImapConfig;
    #[cfg(feature = "maildir-backend")]
    use himalaya_lib::MaildirConfig;
    #[cfg(feature = "notmuch-backend")]
    use himalaya_lib::NotmuchConfig;

    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::account::DeserializedBaseAccountConfig;

    #[cfg(feature = "imap-backend")]
    use crate::account::DeserializedImapAccountConfig;
    #[cfg(feature = "maildir-backend")]
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

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `backend` at line 1 column 1"
        );
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
            .starts_with("unknown variant `bad`"));
    }

    #[test]
    fn account_missing_email_field() {
        let config = make_config(
            "[account]
            backend = \"none\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `email` at line 1 column 1"
        );
    }

    #[test]
    fn imap_account_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"imap\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `imap-host` at line 1 column 1"
        );
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

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `imap-port` at line 1 column 1"
        );
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

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `imap-login` at line 1 column 1"
        );
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

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `imap-passwd-cmd` at line 1 column 1"
        );
    }

    #[test]
    fn account_backend_maildir_missing_root_dir_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            sender = \"none\"
            backend = \"maildir\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `maildir-root-dir` at line 1 column 1"
        );
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

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `notmuch-db-path` at line 1 column 1"
        );
    }

    #[test]
    fn account_missing_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `sender` at line 1 column 1"
        );
    }

    #[test]
    fn account_invalid_sender_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"bad\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "unknown variant `bad`, expected one of `none`, `internal`, `external` at line 1 column 1",
        );
    }

    #[test]
    fn account_internal_sender_missing_host_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"internal\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `smtp-host` at line 1 column 1"
        );
    }

    #[test]
    fn account_internal_sender_missing_port_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"internal\"
            smtp-host = \"localhost\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `smtp-port` at line 1 column 1"
        );
    }

    #[test]
    fn account_internal_sender_missing_login_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"internal\"
            smtp-host = \"localhost\"
            smtp-port = 25",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `smtp-login` at line 1 column 1"
        );
    }

    #[test]
    fn account_internal_sender_missing_passwd_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"internal\"
            smtp-host = \"localhost\"
            smtp-port = 25
            smtp-login = \"login\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `smtp-passwd-cmd` at line 1 column 1"
        );
    }

    #[test]
    fn account_external_sender_missing_cmd_field() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"external\"",
        );

        assert_eq!(
            config.unwrap_err().root_cause().to_string(),
            "missing field `send-cmd` at line 1 column 1"
        );
    }

    #[test]
    fn account_internal_sender_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"internal\"
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
                        email_sender: EmailSender::Internal(SmtpConfig {
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
    fn account_external_sender_minimum_config() {
        let config = make_config(
            "[account]
            email = \"test@localhost\"
            backend = \"none\"
            sender = \"external\"
            send-cmd = \"echo send\"",
        );

        assert_eq!(
            config.unwrap(),
            DeserializedConfig {
                accounts: HashMap::from_iter([(
                    "account".into(),
                    DeserializedAccountConfig::None(DeserializedBaseAccountConfig {
                        email: "test@localhost".into(),
                        email_sender: EmailSender::External(EmailSendCmd {
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
            imap-passwd-cmd = \"echo password\"",
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
                            passwd_cmd: "echo password".into(),
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
