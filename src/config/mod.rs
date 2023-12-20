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
                                watch: c.watch.map(|c| c.remote),
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
