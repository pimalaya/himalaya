use lettre;
use std::{fmt, result};

use crate::config;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    TransportError(lettre::transport::smtp::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(smtp): ")?;
        match self {
            Error::TransportError(err) => err.fmt(f),
        }
    }
}

impl From<lettre::transport::smtp::Error> for Error {
    fn from(err: lettre::transport::smtp::Error) -> Error {
        Error::TransportError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Utils

pub fn send(config: &config::ServerInfo, msg: &lettre::Message) -> Result<()> {
    use lettre::Transport;

    lettre::transport::smtp::SmtpTransport::relay(&config.host)?
        .credentials(config.to_smtp_creds())
        .build()
        .send(msg)
        .map(|_| Ok(()))?
}
