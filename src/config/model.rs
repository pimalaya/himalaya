use error_chain::error_chain;
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use serde::Deserialize;
use std::{collections::HashMap, env, fs::File, io::Read, path::PathBuf};
use toml;

use crate::output::utils::run_cmd;

error_chain! {}

const DEFAULT_PAGE_SIZE: usize = 10;

// Account

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Account {
    // Override
    pub name: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub signature: Option<String>,
    pub default_page_size: Option<usize>,

    // Specific
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

impl Account {
    pub fn imap_addr(&self) -> (&str, u16) {
        (&self.imap_host, self.imap_port)
    }

    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd).chain_err(|| "Cannot run IMAP passwd cmd")?;
        let passwd = passwd.trim_end_matches("\n").to_owned();

        Ok(passwd)
    }

    pub fn imap_starttls(&self) -> bool {
        match self.imap_starttls {
            Some(true) => true,
            _ => false,
        }
    }

    pub fn imap_insecure(&self) -> bool {
        match self.imap_insecure {
            Some(true) => true,
            _ => false,
        }
    }

    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd).chain_err(|| "Cannot run SMTP passwd cmd")?;
        let passwd = passwd.trim_end_matches("\n").to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    pub fn smtp_starttls(&self) -> bool {
        match self.smtp_starttls {
            Some(true) => true,
            _ => false,
        }
    }

    pub fn smtp_insecure(&self) -> bool {
        match self.smtp_insecure {
            Some(true) => true,
            _ => false,
        }
    }
}

// Config

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub name: String,
    pub downloads_dir: Option<PathBuf>,
    pub notify_cmd: Option<String>,
    pub signature: Option<String>,
    pub default_page_size: Option<usize>,

    #[serde(flatten)]
    pub accounts: HashMap<String, Account>,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path =
            env::var("XDG_CONFIG_HOME").chain_err(|| "Cannot find `XDG_CONFIG_HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_xdg_alt() -> Result<PathBuf> {
        let path = env::var("HOME").chain_err(|| "Cannot find `HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push(".config");
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home() -> Result<PathBuf> {
        let path = env::var("HOME").chain_err(|| "Cannot find `HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push(".himalayarc");

        Ok(path)
    }

    pub fn new_from_file() -> Result<Self> {
        let mut file = File::open(
            Self::path_from_xdg()
                .or_else(|_| Self::path_from_xdg_alt())
                .or_else(|_| Self::path_from_home())
                .chain_err(|| "Cannot find config path")?,
        )
        .chain_err(|| "Cannot open config file")?;

        let mut content = vec![];
        file.read_to_end(&mut content)
            .chain_err(|| "Cannot read config file")?;

        Ok(toml::from_slice(&content).chain_err(|| "Cannot parse config file")?)
    }

    pub fn find_account_by_name(&self, name: Option<&str>) -> Result<&Account> {
        match name {
            Some(name) => self
                .accounts
                .get(name)
                .ok_or_else(|| format!("Cannot find account `{}`", name).into()),
            None => self
                .accounts
                .iter()
                .find(|(_, account)| account.default.unwrap_or(false))
                .map(|(_, account)| account)
                .ok_or_else(|| "Cannot find default account".into()),
        }
    }

    pub fn downloads_filepath(&self, account: &Account, filename: &str) -> PathBuf {
        account
            .downloads_dir
            .as_ref()
            .unwrap_or(self.downloads_dir.as_ref().unwrap_or(&env::temp_dir()))
            .to_owned()
            .join(filename)
    }

    pub fn address(&self, account: &Account) -> String {
        let name = account.name.as_ref().unwrap_or(&self.name);
        format!("{} <{}>", name, account.email)
    }

    pub fn run_notify_cmd(&self, subject: &str, sender: &str) -> Result<()> {
        let default_cmd = format!(r#"notify-send "📫 {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|s| format!(r#"{} "{}" "{}""#, s, subject, sender))
            .unwrap_or(default_cmd);

        run_cmd(&cmd).chain_err(|| "Cannot run notify cmd")?;

        Ok(())
    }

    pub fn signature(&self, account: &Account) -> Option<String> {
        account
            .signature
            .as_ref()
            .or_else(|| self.signature.as_ref())
            .map(|sig| sig.to_owned())
    }

    pub fn default_page_size(&self, account: &Account) -> usize {
        account
            .default_page_size
            .as_ref()
            .or_else(|| self.default_page_size.as_ref())
            .or(Some(&DEFAULT_PAGE_SIZE))
            .unwrap()
            .to_owned()
    }
}
