//! Deserialized config module.
//!
//! This module contains the raw deserialized representation of the
//! user configuration file.

use log::{debug, trace};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, io, path::PathBuf};
use thiserror::Error;
use toml;

use super::*;

#[derive(Error, Debug)]
pub enum DeserializeConfigError {
    #[error("cannot read config file")]
    ReadConfigFile(#[source] io::Error),
    #[error("cannot parse config file")]
    ParseConfigFile(#[source] toml::de::Error),
    #[error("cannot read environment variable {1}")]
    ReadEnvVar(#[source] env::VarError, &'static str),
}

/// Represents the user config file.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeserializedConfig {
    /// Represents the display name of the user.
    pub name: String,
    /// Represents the downloads directory (mostly for attachments).
    pub downloads_dir: Option<PathBuf>,
    /// Represents the signature of the user.
    pub signature: Option<String>,
    /// Overrides the default signature delimiter "`-- \n`".
    pub signature_delimiter: Option<String>,
    /// Represents the default page size for listings.
    pub default_page_size: Option<usize>,
    /// Represents the notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new messages
    pub notify_query: Option<String>,
    /// Represents the watch commands.
    pub watch_cmds: Option<Vec<String>>,

    /// Represents all the user accounts.
    #[serde(flatten)]
    pub accounts: HashMap<String, DeserializedAccountConfig>,
}

impl DeserializedConfig {
    /// Tries to create a config from an optional path.
    pub fn from_opt_path(path: Option<&str>) -> Result<Self, DeserializeConfigError> {
        trace!(">> parse config from path");
        debug!("path: {:?}", path);

        let path = path.map(|s| s.into()).unwrap_or(Self::path()?);
        let content = fs::read_to_string(path).map_err(DeserializeConfigError::ReadConfigFile)?;
        let config = toml::from_str(&content).map_err(DeserializeConfigError::ParseConfigFile)?;

        trace!("config: {:?}", config);
        trace!("<< parse config from path");
        Ok(config)
    }

    /// Tries to get the XDG config file path from XDG_CONFIG_HOME
    /// environment variable.
    fn path_from_xdg() -> Result<PathBuf, DeserializeConfigError> {
        let path = env::var("XDG_CONFIG_HOME")
            .map_err(|err| DeserializeConfigError::ReadEnvVar(err, "XDG_CONFIG_HOME"))?;
        let path = PathBuf::from(path).join("himalaya").join("config.toml");
        Ok(path)
    }

    /// Tries to get the XDG config file path from HOME environment
    /// variable.
    fn path_from_xdg_alt() -> Result<PathBuf, DeserializeConfigError> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path =
            env::var(home_var).map_err(|err| DeserializeConfigError::ReadEnvVar(err, home_var))?;
        let path = PathBuf::from(path)
            .join(".config")
            .join("himalaya")
            .join("config.toml");
        Ok(path)
    }

    /// Tries to get the .himalayarc config file path from HOME
    /// environment variable.
    fn path_from_home() -> Result<PathBuf, DeserializeConfigError> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path =
            env::var(home_var).map_err(|err| DeserializeConfigError::ReadEnvVar(err, home_var))?;
        let path = PathBuf::from(path).join(".himalayarc");
        Ok(path)
    }

    /// Tries to get the config file path.
    pub fn path() -> Result<PathBuf, DeserializeConfigError> {
        Self::path_from_xdg()
            .or_else(|_| Self::path_from_xdg_alt())
            .or_else(|_| Self::path_from_home())
    }
}
