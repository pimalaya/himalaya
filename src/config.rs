use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fmt,
    fs::File,
    io::{self, Read},
    path::PathBuf,
    result,
};
use toml;

use crate::output::{self, run_cmd};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    ParseTomlError(toml::de::Error),
    ParseTomlAccountsError,
    GetEnvVarError(env::VarError),
    GetPathNotFoundError,
    GetAccountNotFoundError(String),
    GetAccountDefaultNotFoundError,
    OutputError(output::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "config: ")?;

        match self {
            Error::IoError(err) => err.fmt(f),
            Error::ParseTomlError(err) => err.fmt(f),
            Error::ParseTomlAccountsError => write!(f, "no account found"),
            Error::GetEnvVarError(err) => err.fmt(f),
            Error::GetPathNotFoundError => write!(f, "path not found"),
            Error::GetAccountNotFoundError(account) => write!(f, "account {} not found", account),
            Error::GetAccountDefaultNotFoundError => write!(f, "no default account found"),
            Error::OutputError(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Error {
        Error::ParseTomlError(err)
    }
}

impl From<env::VarError> for Error {
    fn from(err: env::VarError) -> Error {
        Error::GetEnvVarError(err)
    }
}

impl From<output::Error> for Error {
    fn from(err: output::Error) -> Error {
        Error::OutputError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Account

#[derive(Debug, Deserialize)]
pub struct Account {
    // Override
    pub name: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    // Specific
    pub default: Option<bool>,
    pub email: String,

    pub imap_host: String,
    pub imap_port: u16,
    pub imap_login: String,
    pub imap_passwd_cmd: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,
}

impl Account {
    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd)?;
        let passwd = passwd.trim_end_matches("\n").to_owned();

        Ok(passwd)
    }

    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd)?;
        let passwd = passwd.trim_end_matches("\n").to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    pub fn imap_addr(&self) -> (&str, u16) {
        (&self.imap_host, self.imap_port)
    }
}

// Config

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub downloads_dir: Option<PathBuf>,

    #[serde(flatten)]
    pub accounts: HashMap<String, Account>,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path = env::var("XDG_CONFIG_HOME")?;
        let mut path = PathBuf::from(path);
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home() -> Result<PathBuf> {
        let path = env::var("HOME")?;
        let mut path = PathBuf::from(path);
        path.push(".config");
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_tmp() -> Result<PathBuf> {
        let mut path = env::temp_dir();
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    pub fn new_from_file() -> Result<Self> {
        let mut file = File::open(
            Self::path_from_xdg()
                .or_else(|_| Self::path_from_home())
                .or_else(|_| Self::path_from_tmp())
                .or_else(|_| Err(Error::GetPathNotFoundError))?,
        )?;

        let mut content = vec![];
        file.read_to_end(&mut content)?;

        Ok(toml::from_slice(&content)?)
    }

    pub fn find_account_by_name(&self, name: Option<&str>) -> Result<&Account> {
        match name {
            Some(name) => self
                .accounts
                .get(name)
                .ok_or_else(|| Error::GetAccountNotFoundError(name.to_owned())),
            None => self
                .accounts
                .iter()
                .find(|(_, account)| account.default.unwrap_or(false))
                .map(|(_, account)| account)
                .ok_or_else(|| Error::GetAccountDefaultNotFoundError),
        }
    }

    pub fn downloads_filepath(&self, account: &Account, filename: &str) -> PathBuf {
        let temp_dir = env::temp_dir();
        let mut full_path = account
            .downloads_dir
            .as_ref()
            .unwrap_or(self.downloads_dir.as_ref().unwrap_or(&temp_dir))
            .to_owned();

        full_path.push(filename);
        full_path
    }

    pub fn address(&self, account: &Account) -> String {
        let name = account.name.as_ref().unwrap_or(&self.name);
        format!("{} <{}>", name, account.email)
    }
}
