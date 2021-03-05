use imap;
use native_tls::{self, TlsConnector, TlsStream};
use std::{fmt, net::TcpStream, result};

use crate::config::{self, Account};
use crate::mbox::{Mbox, Mboxes};
use crate::msg::{Msg, Msgs};

// Error wrapper

#[derive(Debug)]
pub enum Error {
    CreateTlsConnectorError(native_tls::Error),
    CreateImapSession(imap::Error),
    ParseEmailError(mailparse::MailParseError),
    ReadEmailNotFoundError(String),
    ReadEmailEmptyPartError(String, String),
    ExtractAttachmentsEmptyError(String),
    ConfigError(config::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "imap: ")?;

        match self {
            Error::CreateTlsConnectorError(err) => err.fmt(f),
            Error::CreateImapSession(err) => err.fmt(f),
            Error::ParseEmailError(err) => err.fmt(f),
            Error::ConfigError(err) => err.fmt(f),
            Error::ReadEmailNotFoundError(uid) => {
                write!(f, "no email found for uid {}", uid)
            }
            Error::ReadEmailEmptyPartError(uid, mime) => {
                write!(f, "no {} content found for uid {}", mime, uid)
            }
            Error::ExtractAttachmentsEmptyError(uid) => {
                write!(f, "no attachment found for uid {}", uid)
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

impl From<mailparse::MailParseError> for Error {
    fn from(err: mailparse::MailParseError) -> Error {
        Error::ParseEmailError(err)
    }
}

impl From<config::Error> for Error {
    fn from(err: config::Error) -> Error {
        Error::ConfigError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Imap connector

#[derive(Debug)]
pub struct ImapConnector<'a> {
    pub account: &'a Account,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl<'a> ImapConnector<'a> {
    pub fn new(account: &'a Account) -> Result<Self> {
        let tls = TlsConnector::new()?;
        let client = match account.imap_starttls {
            Some(true) => imap::connect_starttls(account.imap_addr(), &account.imap_host, &tls),
            _ => imap::connect(account.imap_addr(), &account.imap_host, &tls),
        }?;
        let sess = client
            .login(&account.imap_login, &account.imap_passwd()?)
            .map_err(|res| res.0)?;

        Ok(Self { account, sess })
    }

    pub fn logout(&mut self) {
        match self.sess.logout() {
            _ => (),
        }
    }

    pub fn list_mboxes(&mut self) -> Result<Mboxes> {
        let mboxes = self
            .sess
            .list(Some(""), Some("*"))?
            .iter()
            .map(Mbox::from_name)
            .collect::<Vec<_>>();

        Ok(Mboxes(mboxes))
    }

    pub fn list_msgs(&mut self, mbox: &str, page_size: &u32, page: &u32) -> Result<Msgs> {
        let last_seq = self.sess.select(mbox)?.exists;
        let begin = last_seq - page * page_size;
        let end = begin - (begin - 1).min(page_size - 1);
        let range = format!("{}:{}", begin, end);

        let msgs = self
            .sess
            .fetch(range, "(UID ENVELOPE INTERNALDATE)")?
            .iter()
            .rev()
            .map(Msg::from)
            .collect::<Vec<_>>();

        Ok(Msgs(msgs))
    }

    pub fn search_msgs(
        &mut self,
        mbox: &str,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Msgs> {
        self.sess.select(mbox)?;

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let uids = self
            .sess
            .search(query)?
            .iter()
            .map(|seq| seq.to_string())
            .collect::<Vec<_>>();
        let range = uids[begin..end.min(uids.len())].join(",");

        let msgs = self
            .sess
            .fetch(range, "(UID ENVELOPE INTERNALDATE)")?
            .iter()
            .map(Msg::from)
            .collect::<Vec<_>>();

        Ok(Msgs(msgs))
    }

    pub fn read_msg(&mut self, mbox: &str, uid: &str) -> Result<Vec<u8>> {
        self.sess.select(mbox)?;

        match self.sess.uid_fetch(uid, "BODY[]")?.first() {
            None => Err(Error::ReadEmailNotFoundError(uid.to_string())),
            Some(fetch) => Ok(fetch.body().unwrap_or(&[]).to_vec()),
        }
    }

    pub fn append_msg(&mut self, mbox: &str, msg: &[u8]) -> Result<()> {
        use imap::types::Flag::*;
        self.sess.append_with_flags(mbox, msg, &[Seen])?;
        Ok(())
    }
}
