#[cfg(feature = "wizard")]
pub mod wizard;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use color_eyre::{eyre::eyre, Result};
use crossterm::style::Color;
use email::{
    account::config::AccountConfig, envelope::config::EnvelopeConfig, folder::config::FolderConfig,
    message::config::MessageConfig,
};
use pimalaya_tui::config::TomlConfig;
use serde::{Deserialize, Serialize};
use shellexpand_utils::{canonicalize, expand};

use crate::account::config::{ListAccountsTableConfig, TomlAccountConfig};

/// Represents the user config file.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub accounts: HashMap<String, TomlAccountConfig>,
    pub account: Option<AccountsConfig>,
}

impl TomlConfig<AccountConfig> for Config {
    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }
}

impl Config {
    /// Create and save a TOML configuration using the wizard.
    ///
    /// If the user accepts the confirmation, the wizard starts and
    /// help him to create his configuration file. Otherwise the
    /// program stops.
    ///
    /// NOTE: the wizard can only be used with interactive shells.
    #[cfg(feature = "wizard")]
    async fn from_wizard(path: &PathBuf) -> Result<Self> {
        Self::confirm_from_wizard(path)?;
        wizard::configure(path).await
    }

    /// Read and parse the TOML configuration from default paths.
    pub async fn from_default_paths() -> Result<Self> {
        match Self::first_valid_default_path() {
            Some(path) => Self::from_paths(&[path]),
            #[cfg(feature = "wizard")]
            None => Self::from_wizard(&Self::default_path()?).await,
            #[cfg(not(feature = "wizard"))]
            None => color_eyre::eyre::bail!("cannot find config file from default paths"),
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
    pub async fn from_paths_or_default(paths: &[PathBuf]) -> Result<Self> {
        match paths.len() {
            0 => Self::from_default_paths().await,
            _ if paths[0].exists() => Self::from_paths(paths),
            _ => Self::from_wizard(&paths[0]).await,
        }
    }

    pub fn into_toml_account_config(
        &self,
        account_name: Option<&str>,
    ) -> Result<(String, TomlAccountConfig)> {
        #[allow(unused_mut)]
        let (account_name, mut toml_account_config) = match account_name {
            Some("default") | Some("") | None => self
                .accounts
                .iter()
                .find_map(|(name, account)| {
                    account
                        .default
                        .filter(|default| *default)
                        .map(|_| (name.to_owned(), account.clone()))
                })
                .ok_or_else(|| eyre!("cannot find default account")),
            Some(name) => self
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account.clone()))
                .ok_or_else(|| eyre!("cannot find account {name}")),
        }?;

        #[cfg(all(feature = "imap", feature = "keyring"))]
        if let Some(imap_config) = toml_account_config.imap.as_mut() {
            imap_config
                .auth
                .replace_undefined_keyring_entries(&account_name)?;
        }

        #[cfg(all(feature = "smtp", feature = "keyring"))]
        if let Some(smtp_config) = toml_account_config.smtp.as_mut() {
            smtp_config
                .auth
                .replace_undefined_keyring_entries(&account_name)?;
        }

        Ok((account_name, toml_account_config))
    }

    /// Build account configurations from a given account name.
    pub fn into_account_configs(
        self,
        account_name: Option<&str>,
    ) -> Result<(Arc<TomlAccountConfig>, Arc<AccountConfig>)> {
        let (account_name, toml_account_config) = self.into_toml_account_config(account_name)?;

        let config = email::config::Config {
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
                                thread: c.thread.map(|c| c.remote),
                            }),
                            flag: None,
                            message: config.message.map(|c| MessageConfig {
                                read: c.read.map(|c| c.remote),
                                write: c.write.map(|c| c.remote),
                                send: c.send.map(|c| c.remote),
                                delete: c.delete.map(Into::into),
                            }),
                            template: config.template,
                            #[cfg(feature = "pgp")]
                            pgp: config.pgp,
                        },
                    )
                },
            )),
        };

        let account_config = config.account(account_name)?;

        Ok((Arc::new(toml_account_config), Arc::new(account_config)))
    }

    pub fn account_list_table_preset(&self) -> Option<String> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.preset.clone())
    }

    pub fn account_list_table_name_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.name_color)
    }

    pub fn account_list_table_backends_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.backends_color)
    }

    pub fn account_list_table_default_color(&self) -> Option<Color> {
        self.account
            .as_ref()
            .and_then(|account| account.list.as_ref())
            .and_then(|list| list.table.as_ref())
            .and_then(|table| table.default_color)
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountsConfig {
    pub list: Option<ListAccountsConfig>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListAccountsConfig {
    pub table: Option<ListAccountsTableConfig>,
}
