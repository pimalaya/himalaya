use lettre;
use std::{fmt, result};

use crate::config::{self, Account};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    TransportError(lettre::transport::smtp::Error),
    ConfigError(config::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(smtp): ")?;
        match self {
            Error::TransportError(err) => err.fmt(f),
            Error::ConfigError(err) => err.fmt(f),
        }
    }
}

impl From<lettre::transport::smtp::Error> for Error {
    fn from(err: lettre::transport::smtp::Error) -> Error {
        Error::TransportError(err)
    }
}

impl From<config::Error> for Error {
    fn from(err: config::Error) -> Error {
        Error::ConfigError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Utils

pub fn send(account: &Account, msg: &lettre::Message) -> Result<()> {
    use lettre::Transport;

    // TODO
    // lettre::transport::smtp::SmtpTransport::starttls_relay

    lettre::transport::smtp::SmtpTransport::relay(&account.smtp_host)?
        .credentials(account.smtp_creds()?)
        .build()
        .send(msg)
        .map(|_| Ok(()))?
}
