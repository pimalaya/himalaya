use error_chain::error_chain;
use imap;
use native_tls::{self, TlsConnector, TlsStream};
use std::net::TcpStream;

use crate::{
    config::{self, Account, Config},
    mbox::model::{Mbox, Mboxes},
    msg::model::Msg,
};

error_chain! {
    links {
        Config(config::Error, config::ErrorKind);
    }
}

#[derive(Debug)]
pub struct ImapConnector<'a> {
    pub account: &'a Account,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl<'ic> ImapConnector<'ic> {
    pub fn new(account: &'ic Account) -> Result<Self> {
        let tls = TlsConnector::new().chain_err(|| "Cannot create TLS connector")?;
        let client = if account.imap_starttls() {
            imap::connect_starttls(account.imap_addr(), &account.imap_host, &tls)
                .chain_err(|| "Cannot connect using STARTTLS")
        } else {
            imap::connect(account.imap_addr(), &account.imap_host, &tls)
                .chain_err(|| "Cannot connect using TLS")
        }?;
        let sess = client
            .login(&account.imap_login, &account.imap_passwd()?)
            .map_err(|res| res.0)
            .chain_err(|| "Cannot login to IMAP server")?;

        Ok(Self { account, sess })
    }

    pub fn logout(&mut self) {
        match self.sess.logout() {
            _ => (),
        }
    }

    pub fn set_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("FLAGS ({})", flags))
            .chain_err(|| format!("Cannot set flags `{}`", &flags))?;

        Ok(())
    }

    pub fn add_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("+FLAGS ({})", flags))
            .chain_err(|| format!("Cannot add flags `{}`", &flags))?;

        Ok(())
    }

    pub fn remove_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("-FLAGS ({})", flags))
            .chain_err(|| format!("Cannot remove flags `{}`", &flags))?;

        Ok(())
    }

    fn last_new_seq(&mut self) -> Result<Option<u32>> {
        Ok(self
            .sess
            .uid_search("NEW")
            .chain_err(|| "Cannot search new uids")?
            .into_iter()
            .next())
    }

    pub fn idle(&mut self, config: &Config, mbox: &str) -> Result<()> {
        let mut prev_seq = 0;
        self.sess
            .examine(mbox)
            .chain_err(|| format!("Cannot examine mailbox `{}`", mbox))?;

        loop {
            self.sess
                .idle()
                .and_then(|idle| idle.wait_keepalive())
                .chain_err(|| "Cannot wait in IDLE mode")?;

            if let Some(seq) = self.last_new_seq()? {
                if prev_seq != seq {
                    let msgs = self
                        .sess
                        .uid_fetch(seq.to_string(), "(ENVELOPE)")
                        .chain_err(|| "Cannot fetch enveloppe")?;
                    let msg = msgs
                        .iter()
                        .next()
                        .ok_or_else(|| "Cannot fetch first message")
                        .map(Msg::from)?;

                    config.run_notify_cmd(&msg.subject, &msg.sender)?;
                    prev_seq = seq;
                }
            }
        }
    }

    pub fn list_mboxes(&mut self) -> Result<Mboxes> {
        let mboxes = self
            .sess
            .list(Some(""), Some("*"))
            .chain_err(|| "Cannot list mailboxes")?
            .iter()
            .map(Mbox::from_name)
            .collect::<Vec<_>>();

        Ok(Mboxes(mboxes))
    }

    pub fn list_msgs(
        &mut self,
        mbox: &str,
        page_size: &u32,
        page: &u32,
    ) -> Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>> {
        let last_seq = self
            .sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?
            .exists as i64;

        if last_seq == 0 {
            return Err(format!("Cannot select empty mailbox `{}`", mbox).into());
        }

        // TODO: add tests, improve error management when empty page
        let cursor = (page * page_size) as i64;
        let begin = 1.max(last_seq - cursor);
        let end = begin - begin.min(*page_size as i64) + 1;
        let range = format!("{}:{}", begin, end);

        let fetches = self
            .sess
            .fetch(range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .chain_err(|| "Cannot fetch messages")?;

        Ok(fetches)
    }

    pub fn search_msgs(
        &mut self,
        mbox: &str,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<imap::types::ZeroCopy<Vec<imap::types::Fetch>>> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?;

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let uids = self
            .sess
            .search(query)
            .chain_err(|| format!("Cannot search in `{}` with query `{}`", mbox, query))?
            .iter()
            .map(|seq| seq.to_string())
            .collect::<Vec<_>>();
        let range = uids[begin..end.min(uids.len())].join(",");

        let fetches = self
            .sess
            .fetch(&range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .chain_err(|| format!("Cannot fetch range `{}`", &range))?;
        // .iter()
        // .map(|fetch| Msg::from(fetch))
        // .collect::<Vec<_>>();

        Ok(fetches)
    }

    pub fn read_msg(&mut self, mbox: &str, uid: &str) -> Result<Vec<u8>> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Cannot select mailbox `{}`", mbox))?;

        match self
            .sess
            .uid_fetch(uid, "BODY[]")
            .chain_err(|| "Cannot fetch bodies")?
            .first()
        {
            None => Err(format!("Cannot find message `{}`", uid).into()),
            Some(fetch) => Ok(fetch.body().unwrap_or(&[]).to_vec()),
        }
    }

    pub fn append_msg(&mut self, mbox: &str, msg: &[u8]) -> Result<()> {
        self.sess
            .append_with_flags(mbox, msg, &[imap::types::Flag::Seen])
            .chain_err(|| format!("Cannot append message to `{}` with \\Seen flag", mbox))?;

        Ok(())
    }
}
