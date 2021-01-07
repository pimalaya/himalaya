use imap;
use mailparse::{self, MailHeaderMap};
use native_tls::{self, TlsConnector, TlsStream};
use std::{fmt, net::TcpStream, result};

use crate::config;
use crate::email::Email;
use crate::mailbox::Mailbox;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    CreateTlsConnectorError(native_tls::Error),
    CreateImapSession(imap::Error),
    ReadEmailNotFoundError(String),
    ReadEmailEmptyPartError(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(imap): ")?;
        match self {
            Error::CreateTlsConnectorError(err) => err.fmt(f),
            Error::CreateImapSession(err) => err.fmt(f),
            Error::ReadEmailNotFoundError(uid) => {
                write!(f, "no email found for uid {}", uid)
            }
            Error::ReadEmailEmptyPartError(uid, mime) => {
                write!(f, "no {} content found for uid {}", mime, uid)
            }
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Error {
        Error::CreateTlsConnectorError(err)
    }
}

impl From<imap::Error> for Error {
    fn from(err: imap::Error) -> Error {
        Error::CreateImapSession(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Imap connector

#[derive(Debug)]
pub struct ImapConnector {
    pub config: config::ServerInfo,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl ImapConnector {
    fn extract_subparts_by_mime(mime: &str, part: &mailparse::ParsedMail, parts: &mut Vec<String>) {
        match part.subparts.len() {
            0 => {
                if part
                    .get_headers()
                    .get_first_value("content-type")
                    .and_then(|v| if v.starts_with(mime) { Some(()) } else { None })
                    .is_some()
                {
                    // TODO: push part instead of body str
                    parts.push(part.get_body().unwrap_or(String::new()))
                }
            }
            _ => {
                part.subparts
                    .iter()
                    .for_each(|p| Self::extract_subparts_by_mime(mime, p, parts));
            }
        }
    }

    pub fn new(config: config::ServerInfo) -> Result<Self> {
        let tls = TlsConnector::new()?;
        let client = imap::connect(config.get_addr(), &config.host, &tls)?;
        let sess = client
            .login(&config.login, &config.password)
            .map_err(|res| res.0)?;

        Ok(Self { config, sess })
    }

    pub fn list_mailboxes(&mut self) -> Result<Vec<Mailbox<'_>>> {
        let mboxes = self
            .sess
            .list(Some(""), Some("*"))?
            .iter()
            .map(Mailbox::from_name)
            .collect::<Vec<_>>();

        Ok(mboxes)
    }

    pub fn read_emails(&mut self, mbox: &str, query: &str) -> Result<Vec<Email<'_>>> {
        self.sess.select(mbox)?;

        let uids = self
            .sess
            .uid_search(query)?
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let emails = self
            .sess
            .uid_fetch(
                uids[..20.min(uids.len())].join(","),
                "(UID ENVELOPE INTERNALDATE)",
            )?
            .iter()
            .map(Email::from_fetch)
            .collect::<Vec<_>>();

        Ok(emails)
    }

    pub fn read_email(&mut self, mbox: &str, uid: &str, mime: &str) -> Result<String> {
        self.sess.select(mbox)?;

        match self.sess.uid_fetch(uid, "BODY[]")?.first() {
            None => Err(Error::ReadEmailNotFoundError(uid.to_string())),
            Some(fetch) => {
                let email = mailparse::parse_mail(fetch.body().unwrap_or(&[])).unwrap();
                let mut parts = vec![];
                Self::extract_subparts_by_mime(mime, &email, &mut parts);

                if parts.len() == 0 {
                    Err(Error::ReadEmailEmptyPartError(
                        uid.to_string(),
                        mime.to_string(),
                    ))
                } else {
                    Ok(parts.join("\r\n"))
                }
            }
        }
    }
}
