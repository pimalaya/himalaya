//! IMAP backend module.
//!
//! This module contains the definition of the IMAP backend.

use anyhow::{anyhow, Context, Result};
use himalaya_lib::account::{AccountConfig, ImapBackendConfig};
use log::{debug, log_enabled, trace, Level};
use native_tls::{TlsConnector, TlsStream};
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
    net::TcpStream,
    thread,
};

use crate::{
    backends::{
        imap::msg_sort_criterion::SortCriteria, Backend, ImapEnvelope, ImapEnvelopes, ImapMboxes,
    },
    mbox::Mboxes,
    msg::{Envelopes, Msg},
    output::run_cmd,
};

use super::ImapFlags;

type ImapSess = imap::Session<TlsStream<TcpStream>>;

pub struct ImapBackend<'a> {
    account_config: &'a AccountConfig,
    imap_config: &'a ImapBackendConfig,
    sess: Option<ImapSess>,
}

impl<'a> ImapBackend<'a> {
    pub fn new(account_config: &'a AccountConfig, imap_config: &'a ImapBackendConfig) -> Self {
        Self {
            account_config,
            imap_config,
            sess: None,
        }
    }

    fn sess(&mut self) -> Result<&mut ImapSess> {
        if self.sess.is_none() {
            debug!("create TLS builder");
            debug!("insecure: {}", self.imap_config.imap_insecure);
            let builder = TlsConnector::builder()
                .danger_accept_invalid_certs(self.imap_config.imap_insecure)
                .danger_accept_invalid_hostnames(self.imap_config.imap_insecure)
                .build()
                .context("cannot create TLS connector")?;

            debug!("create client");
            debug!("host: {}", self.imap_config.imap_host);
            debug!("port: {}", self.imap_config.imap_port);
            debug!("starttls: {}", self.imap_config.imap_starttls);
            let mut client_builder =
                imap::ClientBuilder::new(&self.imap_config.imap_host, self.imap_config.imap_port);
            if self.imap_config.imap_starttls {
                client_builder.starttls();
            }
            let client = client_builder
                .connect(|domain, tcp| Ok(TlsConnector::connect(&builder, domain, tcp)?))
                .context("cannot connect to IMAP server")?;

            debug!("create session");
            debug!("login: {}", self.imap_config.imap_login);
            debug!("passwd cmd: {}", self.imap_config.imap_passwd_cmd);
            let mut sess = client
                .login(
                    &self.imap_config.imap_login,
                    &self.imap_config.imap_passwd()?,
                )
                .map_err(|res| res.0)
                .context("cannot login to IMAP server")?;
            sess.debug = log_enabled!(Level::Trace);
            self.sess = Some(sess);
        }

        match self.sess {
            Some(ref mut sess) => Ok(sess),
            None => Err(anyhow!("cannot get IMAP session")),
        }
    }

    fn search_new_msgs(&mut self, query: &str) -> Result<Vec<u32>> {
        let uids: Vec<u32> = self
            .sess()?
            .uid_search(query)
            .context("cannot search new messages")?
            .into_iter()
            .collect();
        debug!("found {} new messages", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }

    pub fn notify(&mut self, keepalive: u64, mbox: &str) -> Result<()> {
        debug!("notify");

        debug!("examine mailbox {:?}", mbox);
        self.sess()?
            .examine(mbox)
            .context(format!("cannot examine mailbox {}", mbox))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> = self
            .search_new_msgs(&self.account_config.notify_query)?
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
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
                .search_new_msgs(&self.account_config.notify_query)?
                .into_iter()
                .filter(|uid| -> bool { msgs_set.get(uid).is_none() })
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
                    .uid_fetch(uids, "(UID ENVELOPE)")
                    .context("cannot fetch new messages enveloppe")?;

                for fetch in fetches.iter() {
                    let msg = ImapEnvelope::try_from(fetch)?;
                    let uid = fetch.uid.ok_or_else(|| {
                        anyhow!("cannot retrieve message {}'s UID", fetch.message)
                    })?;

                    let from = msg.sender.to_owned().into();
                    self.account_config.run_notify_cmd(&msg.subject, &from)?;

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

    pub fn watch(&mut self, keepalive: u64, mbox: &str) -> Result<()> {
        debug!("examine mailbox: {}", mbox);

        self.sess()?
            .examine(mbox)
            .context(format!("cannot examine mailbox `{}`", mbox))?;

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

            let cmds = self.account_config.watch_cmds.clone();
            thread::spawn(move || {
                debug!("batch execution of {} cmd(s)", cmds.len());
                cmds.iter().for_each(|cmd| {
                    debug!("running command {:?}…", cmd);
                    let res = run_cmd(cmd);
                    debug!("{:?}", res);
                })
            });

            debug!("end loop");
        }
    }
}

impl<'a> Backend<'a> for ImapBackend<'a> {
    fn add_mbox(&mut self, mbox: &str) -> Result<()> {
        self.sess()?
            .create(mbox)
            .context(format!("cannot create imap mailbox {:?}", mbox))
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        let mboxes: ImapMboxes = self
            .sess()?
            .list(Some(""), Some("*"))
            .context("cannot list mailboxes")?
            .into();
        Ok(Box::new(mboxes))
    }

    fn del_mbox(&mut self, mbox: &str) -> Result<()> {
        self.sess()?
            .delete(mbox)
            .context(format!("cannot delete imap mailbox {:?}", mbox))
    }

    fn get_envelopes(
        &mut self,
        mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let last_seq = self
            .sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?
            .exists as usize;
        debug!("last sequence number: {:?}", last_seq);
        if last_seq == 0 {
            return Ok(Box::new(ImapEnvelopes::default()));
        }

        let range = if page_size > 0 {
            let cursor = page * page_size;
            let begin = 1.max(last_seq - cursor);
            let end = begin - begin.min(page_size) + 1;
            format!("{}:{}", end, begin)
        } else {
            String::from("1:*")
        };
        debug!("range: {:?}", range);

        let fetches = self
            .sess()?
            .fetch(&range, "(ENVELOPE FLAGS INTERNALDATE)")
            .context(format!("cannot fetch messages within range {:?}", range))?;
        let envelopes: ImapEnvelopes = fetches.try_into()?;
        Ok(Box::new(envelopes))
    }

    fn search_envelopes(
        &mut self,
        mbox: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let last_seq = self
            .sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?
            .exists;
        debug!("last sequence number: {:?}", last_seq);
        if last_seq == 0 {
            return Ok(Box::new(ImapEnvelopes::default()));
        }

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let seqs: Vec<String> = if sort.is_empty() {
            self.sess()?
                .search(query)
                .context(format!(
                    "cannot find envelopes in {:?} with query {:?}",
                    mbox, query
                ))?
                .iter()
                .map(|seq| seq.to_string())
                .collect()
        } else {
            let sort: SortCriteria = sort.try_into()?;
            let charset = imap::extensions::sort::SortCharset::Utf8;
            self.sess()?
                .sort(&sort, charset, query)
                .context(format!(
                    "cannot find envelopes in {:?} with query {:?}",
                    mbox, query
                ))?
                .iter()
                .map(|seq| seq.to_string())
                .collect()
        };
        if seqs.is_empty() {
            return Ok(Box::new(ImapEnvelopes::default()));
        }

        let range = seqs[begin..end.min(seqs.len())].join(",");
        let fetches = self
            .sess()?
            .fetch(&range, "(ENVELOPE FLAGS INTERNALDATE)")
            .context(format!("cannot fetch messages within range {:?}", range))?;
        let envelopes: ImapEnvelopes = fetches.try_into()?;
        Ok(Box::new(envelopes))
    }

    fn add_msg(&mut self, mbox: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        let flags: ImapFlags = flags.into();
        self.sess()?
            .append(mbox, msg)
            .flags(<ImapFlags as Into<Vec<imap::types::Flag<'a>>>>::into(flags))
            .finish()
            .context(format!("cannot append message to {:?}", mbox))?;
        let last_seq = self
            .sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?
            .exists;
        Ok(Box::new(last_seq))
    }

    fn get_msg(&mut self, mbox: &str, seq: &str) -> Result<Msg> {
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        let fetches = self
            .sess()?
            .fetch(seq, "(FLAGS INTERNALDATE BODY[])")
            .context(format!("cannot fetch messages {:?}", seq))?;
        let fetch = fetches
            .first()
            .ok_or_else(|| anyhow!("cannot find message {:?}", seq))?;
        let msg_raw = fetch.body().unwrap_or_default().to_owned();
        let mut msg = Msg::from_parsed_mail(
            mailparse::parse_mail(&msg_raw).context("cannot parse message")?,
            self.account_config,
        )?;
        msg.raw = msg_raw;
        Ok(msg)
    }

    fn copy_msg(&mut self, mbox_src: &str, mbox_dst: &str, seq: &str) -> Result<()> {
        let msg = self.get_msg(&mbox_src, seq)?.raw;
        println!("raw: {:?}", String::from_utf8(msg.to_vec()).unwrap());
        self.add_msg(&mbox_dst, &msg, "seen")?;
        Ok(())
    }

    fn move_msg(&mut self, mbox_src: &str, mbox_dst: &str, seq: &str) -> Result<()> {
        let msg = self.get_msg(mbox_src, seq)?.raw;
        self.add_flags(mbox_src, seq, "seen deleted")?;
        self.add_msg(&mbox_dst, &msg, "seen")?;
        Ok(())
    }

    fn del_msg(&mut self, mbox: &str, seq: &str) -> Result<()> {
        self.add_flags(mbox, seq, "deleted")
    }

    fn add_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: ImapFlags = flags.into();
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        self.sess()?
            .store(seq_range, format!("+FLAGS ({})", flags))
            .context(format!("cannot add flags {:?}", &flags))?;
        self.sess()?
            .expunge()
            .context(format!("cannot expunge mailbox {:?}", mbox))?;
        Ok(())
    }

    fn set_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: ImapFlags = flags.into();
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        self.sess()?
            .store(seq_range, format!("FLAGS ({})", flags))
            .context(format!("cannot set flags {:?}", &flags))?;
        Ok(())
    }

    fn del_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: ImapFlags = flags.into();
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        self.sess()?
            .store(seq_range, format!("-FLAGS ({})", flags))
            .context(format!("cannot remove flags {:?}", &flags))?;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut sess) = self.sess {
            debug!("logout from IMAP server");
            sess.logout().context("cannot logout from IMAP server")?;
        }
        Ok(())
    }
}
