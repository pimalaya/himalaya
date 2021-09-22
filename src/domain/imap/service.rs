//! Module related to IMAP servicing.
//!
//! This module exposes a service that can interact with IMAP servers.

use anyhow::{anyhow, Context, Result};
use imap;
use log::{debug, trace};
use native_tls::{self, TlsConnector, TlsStream};
use std::{collections::HashSet, convert::TryFrom, iter::FromIterator, net::TcpStream};

use crate::{
    config::entity::{Account, Config},
    domain::{
        mbox::entity::Mbox,
        msg::{entity::Msg, Envelopes, Flags},
    },
};

type ImapSession = imap::Session<TlsStream<TcpStream>>;
type ImapMboxes = imap::types::ZeroCopy<Vec<imap::types::Name>>;

pub trait ImapServiceInterface {
    fn notify(&mut self, config: &Config, keepalive: u64) -> Result<()>;
    fn watch(&mut self, keepalive: u64) -> Result<()>;
    fn list_mboxes(&mut self) -> Result<ImapMboxes>;
    fn list_msgs(&mut self, page_size: &usize, page: &usize) -> Result<Envelopes>;
    fn search_msgs(&mut self, query: &str, page_size: &usize, page: &usize) -> Result<Envelopes>;
    fn get_msg(&mut self, uid: &str) -> Result<Msg>;
    fn append_msg(&mut self, mbox: &Mbox, msg: &mut Msg) -> Result<()>;
    fn expunge(&mut self) -> Result<()>;
    fn logout(&mut self) -> Result<()>;

    /// Add flags to all messages within the given sequence range.
    fn add_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
    /// Replace flags of all messages within the given sequence range.
    fn set_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
    /// Remove flags from all messages within the given sequence range.
    fn remove_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
}

pub struct ImapService<'a> {
    account: &'a Account,
    mbox: &'a Mbox,
    sess: Option<ImapSession>,
}

impl<'a> ImapService<'a> {
    fn sess(&mut self) -> Result<&mut ImapSession> {
        if let None = self.sess {
            debug!("create TLS builder");
            debug!("insecure: {}", self.account.imap_insecure);
            let builder = TlsConnector::builder()
                .danger_accept_invalid_certs(self.account.imap_insecure)
                .danger_accept_invalid_hostnames(self.account.imap_insecure)
                .build()
                .context("cannot create TLS connector")?;

            debug!("create client");
            debug!("host: {}", self.account.imap_host);
            debug!("port: {}", self.account.imap_port);
            debug!("starttls: {}", self.account.imap_starttls);
            let mut client_builder =
                imap::ClientBuilder::new(&self.account.imap_host, self.account.imap_port);
            if self.account.imap_starttls {
                client_builder.starttls();
            }
            let client = client_builder
                .connect(|domain, tcp| Ok(TlsConnector::connect(&builder, domain, tcp)?))
                .context("cannot connect to IMAP server")?;

            debug!("create session");
            debug!("login: {}", self.account.imap_login);
            debug!("passwd cmd: {}", self.account.imap_passwd_cmd);
            self.sess = Some(
                client
                    .login(&self.account.imap_login, &self.account.imap_passwd()?)
                    .map_err(|res| res.0)
                    .context("cannot login to IMAP server")?,
            );
        }

        match self.sess {
            Some(ref mut sess) => Ok(sess),
            None => Err(anyhow!("cannot get IMAP session")),
        }
    }

    fn search_new_msgs(&mut self) -> Result<Vec<u32>> {
        let uids: Vec<u32> = self
            .sess()?
            .uid_search("NEW")
            .context("cannot search new messages")?
            .into_iter()
            .collect();
        debug!("found {} new messages", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }
}

impl<'a> ImapServiceInterface for ImapService<'a> {
    fn list_mboxes(&mut self) -> Result<ImapMboxes> {
        let mboxes = self
            .sess()?
            .list(Some(""), Some("*"))
            .context("cannot list mailboxes")?;
        Ok(mboxes)
    }

    fn list_msgs(&mut self, page_size: &usize, page: &usize) -> Result<Envelopes> {
        let mbox = self.mbox.to_owned();
        let last_seq = self
            .sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?
            .exists as i64;

        if last_seq == 0 {
            return Ok(Envelopes::default());
        }

        // TODO: add tests, improve error management when empty page
        let range = if *page_size > 0 {
            let cursor = (page * page_size) as i64;
            let begin = 1.max(last_seq - cursor);
            let end = begin - begin.min(*page_size as i64) + 1;
            format!("{}:{}", begin, end)
        } else {
            String::from("1:*")
        };

        let fetches = self
            .sess()?
            .fetch(range, "(ENVELOPE FLAGS INTERNALDATE)")
            .context(r#"cannot fetch messages within range "{}""#)?;

        Ok(Envelopes::try_from(fetches)?)
    }

    fn search_msgs(&mut self, query: &str, page_size: &usize, page: &usize) -> Result<Envelopes> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let seqs: Vec<String> = self
            .sess()?
            .search(query)
            .context(format!(
                r#"cannot search in "{}" with query: "{}""#,
                self.mbox.name, query
            ))?
            .iter()
            .map(|seq| seq.to_string())
            .collect();

        if seqs.is_empty() {
            return Ok(Envelopes::default());
        }

        // FIXME: panic if begin > end
        let range = seqs[begin..end.min(seqs.len())].join(",");
        let fetches = self
            .sess()?
            .fetch(&range, "(ENVELOPE FLAGS INTERNALDATE)")
            .context(r#"cannot fetch messages within range "{}""#)?;

        Ok(Envelopes::try_from(fetches)?)
    }

    /// Get the message according to the given `mbox` and `uid`.
    fn get_msg(&mut self, uid: &str) -> Result<Msg> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mbox `{}`", self.mbox.name))?;
        match self
            .sess()?
            .uid_fetch(uid, "(FLAGS BODY[] ENVELOPE INTERNALDATE)")
            .context("cannot fetch bodies")?
            .first()
        {
            None => Err(anyhow!("cannot find message `{}`", uid)),
            Some(fetch) => Ok(Msg::try_from(fetch)?),
        }
    }

    fn append_msg(&mut self, mbox: &Mbox, msg: &mut Msg) -> Result<()> {
        // let body = msg.into_bytes()?;
        // let flags: HashSet<imap::types::Flag<'static>> = (*msg.flags).clone();
        // self.sess()?
        //     .append(&mbox.name, &body)
        //     .flags(flags)
        //     .finish()
        //     .context(format!("cannot append message to `{}`", mbox.name))?;
        Ok(())
    }

    fn notify(&mut self, config: &Config, keepalive: u64) -> Result<()> {
        let mbox = self.mbox.to_owned();

        debug!("examine mailbox: {}", mbox.name);
        self.sess()?
            .examine(&mbox.name)
            .context(format!("cannot examine mailbox `{}`", &self.mbox.name))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> =
            HashSet::from_iter(self.search_new_msgs()?.iter().cloned());
        trace!("messages hashset: {:?}", msgs_set);

        loop {
            debug!("begin loop");
            self.sess()?
                .idle()
                .and_then(|mut idle| {
                    idle.set_keepalive(std::time::Duration::new(keepalive, 0));
                    idle.wait_keepalive_while(|res| {
                        // TODO: handle response
                        trace!("idle response: {:?}", res);
                        false
                    })
                })
                .context("cannot start the idle mode")?;

            let uids: Vec<u32> = self
                .search_new_msgs()?
                .into_iter()
                .filter(|uid| msgs_set.get(&uid).is_none())
                .collect();
            debug!("found {} new messages not in hashset", uids.len());
            trace!("messages hashet: {:?}", msgs_set);

            if !uids.is_empty() {
                let uids = uids
                    .iter()
                    .map(|uid| uid.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let fetches = self
                    .sess()?
                    .uid_fetch(uids, "(ENVELOPE)")
                    .context("cannot fetch new messages enveloppe")?;

                for fetch in fetches.iter() {
                    let msg = Msg::try_from(fetch)?;
                    let uid = fetch.uid.ok_or_else(|| {
                        anyhow!("cannot retrieve message {}'s UID", fetch.message)
                    })?;

                    let subject = msg.headers.subject.clone().unwrap_or_default();
                    config.run_notify_cmd(&subject, &msg.headers.from[0])?;

                    debug!("notify message: {}", uid);
                    trace!("message: {:?}", msg);

                    debug!("insert message {} in hashset", uid);
                    msgs_set.insert(uid);
                    trace!("messages hashset: {:?}", msgs_set);
                }
            }

            debug!("end loop");
        }
    }

    fn watch(&mut self, keepalive: u64) -> Result<()> {
        debug!("examine mailbox: {}", &self.mbox.name);
        let mbox = self.mbox.to_owned();

        self.sess()?
            .examine(&mbox.name)
            .context(format!("cannot examine mailbox `{}`", &self.mbox.name))?;

        loop {
            debug!("begin loop");
            self.sess()?
                .idle()
                .and_then(|mut idle| {
                    idle.set_keepalive(std::time::Duration::new(keepalive, 0));
                    idle.wait_keepalive_while(|res| {
                        // TODO: handle response
                        trace!("idle response: {:?}", res);
                        false
                    })
                })
                .context("cannot start the idle mode")?;
            // FIXME
            // ctx.config.exec_watch_cmds(&ctx.account)?;
            debug!("end loop");
        }
    }

    fn logout(&mut self) -> Result<()> {
        if let Some(ref mut sess) = self.sess {
            debug!("logout from IMAP server");
            sess.logout().context("cannot logout from IMAP server")?;
        }
        Ok(())
    }

    fn add_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        let flags: String = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(seq_range, format!("+FLAGS ({})", flags))
            .context(format!(r#"cannot add flags "{}""#, &flags))?;
        Ok(())
    }

    fn set_flags(&mut self, uid_seq: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(uid_seq, format!("FLAGS ({})", flags))
            .context(format!(r#"cannot set flags "{}""#, &flags))?;
        Ok(())
    }

    fn remove_flags(&mut self, uid_seq: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        let flags = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(uid_seq, format!("-FLAGS ({})", flags))
            .context(format!(r#"cannot remove flags "{}""#, &flags))?;
        Ok(())
    }

    fn expunge(&mut self) -> Result<()> {
        self.sess()?
            .expunge()
            .context(format!(r#"cannot expunge mailbox "{}""#, self.mbox.name))?;
        Ok(())
    }
}

impl<'a> From<(&'a Account, &'a Mbox)> for ImapService<'a> {
    fn from((account, mbox): (&'a Account, &'a Mbox)) -> Self {
        Self {
            account,
            mbox,
            sess: None,
        }
    }
}
