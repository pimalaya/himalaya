use anyhow::{Context, Error, Result};
use log::{debug, trace};
use serde::Deserialize;
use std::{collections::HashMap, convert::TryFrom, env, fs, path::PathBuf};
use toml;

use crate::output::run_cmd;

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIG_DELIM: &str = "-- \n";

/// Represent the user config.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Defines the full display name of the user.
    pub name: String,
    /// Defines the downloads directory (eg. for attachments).
    pub downloads_dir: Option<PathBuf>,
    /// Overrides the default signature delimiter "`--\n `".
    pub signature_delimiter: Option<String>,
    /// Defines the signature.
    pub signature: Option<String>,
    /// Defines the default page size for listings.
    pub default_page_size: Option<usize>,
    /// Defines the inbox folder name.
    pub inbox_folder: Option<String>,
    /// Defines the sent folder name.
    pub sent_folder: Option<String>,
    /// Defines the draft folder name.
    pub draft_folder: Option<String>,
    /// Defines the notify command.
    pub notify_cmd: Option<String>,
    /// Customizes the IMAP query used to fetch new messages.
    pub notify_query: Option<String>,
    /// Defines the watch commands.
    pub watch_cmds: Option<Vec<String>>,

    #[serde(flatten)]
    pub accounts: ConfigAccountsMap,
}

/// Represent the accounts section of the config.
pub type ConfigAccountsMap = HashMap<String, ConfigAccountEntry>;

/// Represent an account in the accounts section.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigAccountEntry {
    pub name: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub signature_delimiter: Option<String>,
    pub signature: Option<String>,
    pub default_page_size: Option<usize>,
    /// Defines a specific inbox folder name for this account.
    pub inbox_folder: Option<String>,
    /// Defines a specific sent folder name for this account.
    pub sent_folder: Option<String>,
    /// Defines a specific draft folder name for this account.
    pub draft_folder: Option<String>,
    /// Customizes the IMAP query used to fetch new messages.
    pub notify_query: Option<String>,
    pub watch_cmds: Option<Vec<String>>,
    pub default: Option<bool>,
    pub email: String,

    pub imap_host: String,
    pub imap_port: u16,
    pub imap_starttls: Option<bool>,
    pub imap_insecure: Option<bool>,
    pub imap_login: String,
    pub imap_passwd_cmd: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: Option<bool>,
    pub smtp_insecure: Option<bool>,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path = env::var("XDG_CONFIG_HOME").context("cannot find `XDG_CONFIG_HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_xdg_alt() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let mut path: PathBuf = env::var(home_var)
            .context(format!("cannot find `{}` env var", home_var))?
            .into();
        path.push(".config");
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let mut path: PathBuf = env::var(home_var)
            .context(format!("cannot find `{}` env var", home_var))?
            .into();
        path.push(".himalayarc");

        Ok(path)
    }

    pub fn path() -> Result<PathBuf> {
        let path = Self::path_from_xdg()
            .or_else(|_| Self::path_from_xdg_alt())
            .or_else(|_| Self::path_from_home())
            .context("cannot find config path")?;

        Ok(path)
    }

    pub fn run_notify_cmd<S: AsRef<str>>(&self, subject: S, sender: S) -> Result<()> {
        let subject = subject.as_ref();
        let sender = sender.as_ref();

        let default_cmd = format!(r#"notify-send "New message from {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|cmd| format!(r#"{} {:?} {:?}"#, cmd, subject, sender))
            .unwrap_or(default_cmd);

        debug!("run command: {}", cmd);
        run_cmd(&cmd).context("cannot run notify cmd")?;
        Ok(())
    }
}

impl TryFrom<Option<&str>> for Config {
    type Error = Error;

    fn try_from(path: Option<&str>) -> Result<Self, Self::Error> {
        debug!("init config from `{:?}`", path);
        let path = path.map(|s| s.into()).unwrap_or(Config::path()?);
        let content = fs::read_to_string(path).context("cannot read config file")?;
        let config = toml::from_str(&content).context("cannot parse config file")?;
        trace!("{:#?}", config);
        Ok(config)
    }
}
