use lettre::transport::smtp::authentication::Credentials;
use serde::Deserialize;
use std::{
    env, fmt,
    fs::File,
    io::{self, Read},
    path::PathBuf,
    result,
};
use toml;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    ParseTomlError(toml::de::Error),
    GetEnvVarError(env::VarError),
    GetPathNotFoundError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(config): ")?;
        match self {
            Error::IoError(err) => err.fmt(f),
            Error::ParseTomlError(err) => err.fmt(f),
            Error::GetEnvVarError(err) => err.fmt(f),
            Error::GetPathNotFoundError => write!(f, "path not found"),
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

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Config

#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub host: String,
    pub port: u16,
    pub login: String,
    pub password: String,
}

impl ServerInfo {
    pub fn get_addr(&self) -> (&str, u16) {
        (&self.host, self.port)
    }

    pub fn to_smtp_creds(&self) -> Credentials {
        Credentials::new(self.login.to_owned(), self.password.to_owned())
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub email: String,
    pub imap: ServerInfo,
    pub smtp: ServerInfo,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path = env::var("XDG_CONFIG_HOME")?;
        let mut path = PathBuf::from(path);
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home(_err: Error) -> Result<PathBuf> {
        let path = env::var("HOME")?;
        let mut path = PathBuf::from(path);
        path.push(".config");
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_tmp(_err: Error) -> Result<PathBuf> {
        let mut path = env::temp_dir();
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    pub fn new_from_file() -> Result<Self> {
        let mut file = File::open(
            Self::path_from_xdg()
                .or_else(Self::path_from_home)
                .or_else(Self::path_from_tmp)
                .or_else(|_| Err(Error::GetPathNotFoundError))?,
        )?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(toml::from_str(&content)?)
    }

    pub fn email_full(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}
