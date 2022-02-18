//! IMAP backend module.
//!
//! This module contains the definition of the IMAP backend.

use anyhow::{anyhow, Context, Result};
use log::{debug, log_enabled, trace, Level};
use native_tls::{TlsConnector, TlsStream};
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
    net::TcpStream,
    thread,
};

use crate::{
    backends::{imap::SortCriteria, Backend},
    config::{AccountConfig, ImapBackendConfig},
    domain::{Envelope, Envelopes, Flags, Msg, RawEnvelopes},
    output::run_cmd,
    Mboxes, RawMboxes,
};

type ImapSess = imap::Session<TlsStream<TcpStream>>;

pub struct ImapBackend<'a> {
    account_config: &'a AccountConfig,
    imap_config: &'a ImapBackendConfig,
    sess: Option<ImapSess>,
    /// Holds raw mailboxes fetched by the `imap` crate in order to
    /// extend mailboxes lifetime outside of handlers. Without that,
    /// it would be impossible for handlers to return a `Mbox` struct
    /// or a `Mboxes` struct due to the `ZeroCopy` constraint.
    _raw_mboxes_cache: Option<RawMboxes>,
    _raw_msgs_cache: Option<RawEnvelopes>,
}

impl<'a> ImapBackend<'a> {
    pub fn new(account_config: &'a AccountConfig, imap_config: &'a ImapBackendConfig) -> Self {
        Self {
            account_config,
            imap_config,
            sess: None,
            _raw_mboxes_cache: None,
            _raw_msgs_cache: None,
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

    fn search_new_msgs(&mut self, account: &AccountConfig) -> Result<Vec<u32>> {
        let uids: Vec<u32> = self
            .sess()?
            .uid_search(&account.notify_query)
            .context("cannot search new messages")?
            .into_iter()
            .collect();
        debug!("found {} new messages", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }

    pub fn notify(&mut self, keepalive: u64, mbox: &str, config: &AccountConfig) -> Result<()> {
        debug!("notify");

        debug!("examine mailbox {:?}", mbox);
        self.sess()?
            .examine(mbox)
            .context(format!("cannot examine mailbox {}", mbox))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> = self
            .search_new_msgs(config)?
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
                .search_new_msgs(config)?
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
                    let msg = Envelope::try_from(fetch)?;
                    let uid = fetch.uid.ok_or_else(|| {
                        anyhow!("cannot retrieve message {}'s UID", fetch.message)
                    })?;

                    let from = msg.sender.to_owned().into();
                    config.run_notify_cmd(&msg.subject, &from)?;

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

    pub fn watch(&mut self, keepalive: u64, mbox: &str, account: &AccountConfig) -> Result<()> {
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

            let cmds = account.watch_cmds.clone();
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
    fn get_mboxes(&mut self) -> Result<Mboxes> {
        let raw_mboxes = self
            .sess()?
            .list(Some(""), Some("*"))
            .context("cannot list mailboxes")?;
        self._raw_mboxes_cache = Some(raw_mboxes);
        Ok(Mboxes::from(self._raw_mboxes_cache.as_ref().unwrap()))
    }

    fn get_envelopes(
        &mut self,
        mbox: &str,
        sort: &str,
        filter: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;

        let sort: SortCriteria = sort.try_into()?;
        let charset = imap::extensions::sort::SortCharset::Utf8;
        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let seqs: Vec<String> = self
            .sess()?
            .sort(&sort, charset, filter)
            .context(format!(
                "cannot search in {:?} with query {:?}",
                mbox, filter
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
            .context(format!("cannot fetch messages within range {:?}", range))?;
        self._raw_msgs_cache = Some(fetches);
        Envelopes::try_from(self._raw_msgs_cache.as_ref().unwrap())
    }

    fn add_msg(&mut self, mbox: &str, msg: &[u8], flags: &str) -> Result<String> {
        let flags: Flags = flags.split_whitespace().collect::<Vec<_>>().try_into()?;
        self.sess()?
            .append(mbox, msg)
            .flags(flags.0)
            .finish()
            .context(format!("cannot append message to {}", mbox))?;
        Ok(String::new())
    }

    fn get_msg(&mut self, mbox: &str, seq: &str) -> Result<Msg> {
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        let fetches = self
            .sess()?
            .fetch(seq, "(ENVELOPE FLAGS INTERNALDATE BODY[])")
            .context(format!("cannot fetch messages {:?}", seq))?;
        let fetch = fetches
            .first()
            .ok_or_else(|| anyhow!("cannot find message {:?}", seq))?;

        Msg::try_from((self.account_config, fetch))
    }

    fn copy_msg(&mut self, mbox_source: &str, mbox_target: &str, seq: &str) -> Result<()> {
        let msg = self.get_msg(&mbox_source, seq)?.raw;
        self.add_msg(&mbox_target, &msg, "seen")?;
        Ok(())
    }

    fn move_msg(&mut self, mbox_src: &str, mbox_dest: &str, seq: &str) -> Result<()> {
        let msg = self.get_msg(mbox_src, seq)?.raw;
        self.add_flags(mbox_src, seq, "seen deleted")?;
        self.add_msg(&mbox_dest, &msg, "seen")?;
        Ok(())
    }

    fn del_msg(&mut self, mbox: &str, seq: &str) -> Result<()> {
        self.add_flags(mbox, seq, "deleted")
    }

    fn add_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: Flags = flags.split_whitespace().collect::<Vec<_>>().try_into()?;
        let flags = flags.to_string();
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
        let flags: Flags = flags.split_whitespace().collect::<Vec<_>>().try_into()?;
        let flags = flags.to_string();
        self.sess()?
            .select(mbox)
            .context(format!("cannot select mailbox {:?}", mbox))?;
        self.sess()?
            .store(seq_range, format!("FLAGS ({})", flags))
            .context(format!("cannot set flags {:?}", &flags))?;
        Ok(())
    }

    fn del_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: Flags = flags.split_whitespace().collect::<Vec<_>>().try_into()?;
        let flags = flags.to_string();
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
