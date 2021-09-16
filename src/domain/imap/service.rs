//! Module related to IMAP servicing.
//!
//! This module exposes a service that can interact with IMAP servers.

use anyhow::{anyhow, Context, Result};
use imap;
use log::{debug, trace};
use native_tls::{self, TlsConnector, TlsStream};
use std::{collections::HashSet, convert::TryFrom, iter::FromIterator, net::TcpStream};

use crate::{
    domain::{
        account::entity::Account, config::entity::Config, mbox::entity::Mbox, msg::entity::Msg,
    },
    flag::model::Flags,
};

type ImapSession = imap::Session<TlsStream<TcpStream>>;
type ImapMsgs = imap::types::ZeroCopy<Vec<imap::types::Fetch>>;
type ImapMboxes = imap::types::ZeroCopy<Vec<imap::types::Name>>;

pub trait ImapServiceInterface {
    fn notify(&mut self, config: &Config, keepalive: u64) -> Result<()>;
    fn watch(&mut self, keepalive: u64) -> Result<()>;
    fn list_mboxes(&mut self) -> Result<ImapMboxes>;
    fn list_msgs(&mut self, page_size: &usize, page: &usize) -> Result<Option<ImapMsgs>>;
    fn search_msgs(
        &mut self,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Option<ImapMsgs>>;
    fn get_msg(&mut self, uid: &str) -> Result<Msg>;
    fn append_msg(&mut self, mbox: &Mbox, msg: &mut Msg) -> Result<()>;
    fn add_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()>;
    fn set_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()>;
    fn remove_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()>;
    fn expunge(&mut self) -> Result<()>;
    fn logout(&mut self) -> Result<()>;
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

    fn list_msgs(&mut self, page_size: &usize, page: &usize) -> Result<Option<ImapMsgs>> {
        let mbox = self.mbox.to_owned();
        let last_seq = self
            .sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox `{}`", self.mbox.name))?
            .exists as i64;

        if last_seq == 0 {
            return Ok(None);
        }

        // TODO: add tests, improve error management when empty page
        let range = if page_size > &0 {
            let cursor = (page * page_size) as i64;
            let begin = 1.max(last_seq - cursor);
            let end = begin - begin.min(*page_size as i64) + 1;
            format!("{}:{}", begin, end)
        } else {
            String::from("1:*")
        };

        let fetches = self
            .sess()?
            .fetch(range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .context("cannot fetch messages")?;

        Ok(Some(fetches))
    }

    fn search_msgs(
        &mut self,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Option<ImapMsgs>> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox `{}`", self.mbox.name))?;

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let uids: Vec<String> = self
            .sess()?
            .search(query)
            .context(format!(
                "cannot search in `{}` with query `{}`",
                self.mbox.name, query
            ))?
            .iter()
            .map(|seq| seq.to_string())
            .collect();

        if uids.is_empty() {
            return Ok(None);
        }

        let range = uids[begin..end.min(uids.len())].join(",");
        let fetches = self
            .sess()?
            .fetch(&range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .context(format!("cannot fetch range `{}`", &range))?;

        Ok(Some(fetches))
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
        let body = msg.into_bytes()?;
        let flags: HashSet<imap::types::Flag<'static>> = (*msg.flags).clone();
        self.sess()?
            .append(&mbox.name, &body)
            .flags(flags)
            .finish()
            .context(format!("cannot append message to `{}`", mbox.name))?;
        Ok(())
    }

    /// Add the given flags to the given mail.
    ///
    /// # Example
    /// ```no_run
    /// use himalaya::imap::model::ImapConnector;
    /// use himalaya::config::model::Account;
    /// use himalaya::flag::model::Flags;
    /// use imap::types::Flag;
    ///
    /// fn main() {
    ///     let account = Account::default();
    ///     let mut imap_conn = ImapConnector::new(&account).unwrap();
    ///     let flags = Flags::from(vec![Flag::Seen]);
    ///
    ///     // Mark the message with the UID 42 in the mailbox "rofl" as "Seen"
    ///     imap_conn.add_flags("rofl", "42", flags).unwrap();
    ///
    ///     imap_conn.logout();
    /// }
    /// ```
    fn add_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        let flags: String = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mbox `{}`", self.mbox.name))?;
        self.sess()?
            .uid_store(uid_seq, format!("+FLAGS ({})", flags))
            .context(format!("cannot add flags `{}`", &flags))?;
        Ok(())
    }

    /// Applies the given flags to the msg.
    ///
    /// # Example
    /// ```no_run
    /// use himalaya::imap::model::ImapConnector;
    /// use himalaya::config::model::Account;
    /// use himalaya::flag::model::Flags;
    /// use imap::types::Flag;
    ///
    /// fn main() {
    ///     let account = Account::default();
    ///     let mut imap_conn = ImapConnector::new(&account).unwrap();
    ///     let flags = Flags::from(vec![Flag::Seen]);
    ///
    ///     // Mark the message with the UID 42 in the mailbox "rofl" as "Seen" and wipe all other
    ///     // flags
    ///     imap_conn.set_flags("rofl", "42", flags).unwrap();
    ///
    ///     imap_conn.logout();
    /// }
    /// ```
    fn set_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        let flags: String = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox `{}`", self.mbox.name))?;
        self.sess()?
            .uid_store(uid_seq, format!("FLAGS ({})", flags))
            .context(format!("cannot set flags `{}`", &flags))?;
        Ok(())
    }

    /// Remove the flags to the message by the given information. Take a look on the example above.
    /// It's pretty similar.
    fn remove_flags(&mut self, uid_seq: &str, flags: Flags) -> Result<()> {
        let mbox = self.mbox.to_owned();
        let flags = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox `{}`", self.mbox.name))?;
        self.sess()?
            .uid_store(uid_seq, format!("-FLAGS ({})", flags))
            .context(format!("cannot remove flags `{}`", &flags))?;
        Ok(())
    }

    fn expunge(&mut self) -> Result<()> {
        self.sess()?
            .expunge()
            .context(format!("cannot expunge `{}`", self.mbox.name))?;
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
                        anyhow!(format!("cannot retrieve message {}'s UID", fetch.message))
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
        debug!("logout from IMAP server");
        self.sess()?
            .logout()
            .context("cannot logout from IMAP server")?;
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
