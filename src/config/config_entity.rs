use anyhow::{Context, Error, Result};
use log::{debug, info, trace};
use std::{collections::HashMap, convert::TryFrom, env, fs, path::PathBuf};
use toml;

use crate::output::run_cmd;

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIG_DELIM: &str = "-- \n";

/// Represents the user deserialized config file.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Represents the display name of the user.
    pub name: String,
    /// Represents the downloads directory (mostly for attachments).
    pub downloads_dir: Option<PathBuf>,
    /// Represents the signature.
    pub signature: Option<String>,
    /// Overrides the default signature delimiter "`--\n `".
    pub signature_delimiter: Option<String>,
    /// Represents the default page size for listings.
    pub default_page_size: Option<usize>,
    /// Overrides the default inbox folder name "INBOX".
    pub inbox_folder: Option<String>,
    /// Overrides the default sent folder name "Sent".
    pub sent_folder: Option<String>,
    /// Overrides the default draft folder name "Drafts".
    pub draft_folder: Option<String>,
    /// Represents the notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new messages
    pub notify_query: Option<String>,
    /// Represents the watch commands.
    pub watch_cmds: Option<Vec<String>>,

    /// Represents all the user accounts.
    #[serde(flatten)]
    pub accounts: HashMap<String, ConfigAccount>,
}

/// Represents all existing kind of account (backend).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ConfigAccount {
    Imap(ConfigImapAccount),
}

/// Represents an IMAP account.
#[derive(Debug, Default, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigImapAccount {
    /// Overrides the display name of the user for this account.
    pub name: Option<String>,
    /// Overrides the downloads directory (mostly for attachments).
    pub downloads_dir: Option<PathBuf>,
    /// Overrides the signature for this account.
    pub signature: Option<String>,
    /// Overrides the signature delimiter for this account.
    pub signature_delimiter: Option<String>,
    /// Overrides the default page size for this account.
    pub default_page_size: Option<usize>,
    /// Overrides the inbox folder name for this account.
    pub inbox_folder: Option<String>,
    /// Overrides the sent folder name for this account.
    pub sent_folder: Option<String>,
    /// Overrides the draft folder name for this account.
    pub draft_folder: Option<String>,
    /// Overrides the notify command for this account.
    pub notify_cmd: Option<String>,
    /// Overrides the IMAP query used to fetch new messages for this account.
    pub notify_query: Option<String>,
    /// Overrides the watch commands for this account.
    pub watch_cmds: Option<Vec<String>>,

    /// Makes this account the default one.
    pub default: Option<bool>,
    /// Represents the account email address.
    pub email: String,

    /// Represents the IMAP host.
    pub imap_host: String,
    /// Represents the IMAP port.
    pub imap_port: u16,
    /// Enables StartTLS.
    pub imap_starttls: Option<bool>,
    /// Trusts any certificate.
    pub imap_insecure: Option<bool>,
    /// Represents the IMAP login.
    pub imap_login: String,
    /// Represents the IMAP password command.
    pub imap_passwd_cmd: String,

    /// Represents the SMTP host.
    pub smtp_host: String,
    /// Represents the SMTP port.
    pub smtp_port: u16,
    /// Enables StartTLS.
    pub smtp_starttls: Option<bool>,
    /// Trusts any certificate.
    pub smtp_insecure: Option<bool>,
    /// Represents the SMTP login.
    pub smtp_login: String,
    /// Represents the SMTP password command.
    pub smtp_passwd_cmd: String,

    /// Represents the command used to encrypt a message.
    pub pgp_encrypt_cmd: Option<String>,
    /// Represents the command used to decrypt a message.
    pub pgp_decrypt_cmd: Option<String>,
}

impl Config {
    /// Tries to get the XDG config file path from XDG_CONFIG_HOME environment variable.
    fn path_from_xdg() -> Result<PathBuf> {
        let path =
            env::var("XDG_CONFIG_HOME").context("cannot find \"XDG_CONFIG_HOME\" env var")?;
        let path = PathBuf::from(path).join("himalaya").join("config.toml");
        Ok(path)
    }

    /// Tries to get the XDG config file path from HOME environment variable.
    fn path_from_xdg_alt() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path = env::var(home_var).context(format!("cannot find {:?} env var", home_var))?;
        let path = PathBuf::from(path)
            .join(".config")
            .join("himalaya")
            .join("config.toml");
        Ok(path)
    }

    /// Tries to get the .himalayarc config file path from HOME environment variable.
    fn path_from_home() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let path = env::var(home_var).context(format!("cannot find {:?} env var", home_var))?;
        let path = PathBuf::from(path).join(".himalayarc");
        Ok(path)
    }

    /// Tries to get the config file path.
    pub fn path() -> Result<PathBuf> {
        Self::path_from_xdg()
            .or_else(|_| Self::path_from_xdg_alt())
            .or_else(|_| Self::path_from_home())
            .context("cannot find config path")
    }

    /// Runs the notify command.
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

/// Tries to create a config from an optional string slice (path from args).
impl TryFrom<Option<&str>> for Config {
    type Error = Error;

    fn try_from(path: Option<&str>) -> Result<Self, Self::Error> {
        info!("begin: trying to parse config from path");
        debug!("path: {:?}", path);
        let path = path.map(|s| s.into()).unwrap_or(Config::path()?);
        let content = fs::read_to_string(path).context("cannot read config file")?;
        let config = toml::from_str(&content).context("cannot parse config file")?;
        info!("end: trying to parse config from path");
        trace!("config: {:?}", config);
        Ok(config)
    }
}
