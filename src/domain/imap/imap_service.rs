//! Module related to IMAP servicing.
//!
//! This module exposes a service that can interact with IMAP servers.

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
    config::{Account, Config},
    domain::{Envelope, Envelopes, Flags, Mbox, Mboxes, Msg, RawEnvelopes, RawMboxes},
    output::run_cmd,
};

type ImapSession = imap::Session<TlsStream<TcpStream>>;

pub trait ImapServiceInterface<'a> {
    fn notify(&mut self, config: &Config, account: &Account, keepalive: u64) -> Result<()>;
    fn watch(&mut self, account: &Account, keepalive: u64) -> Result<()>;
    fn fetch_mboxes(&'a mut self) -> Result<Mboxes>;
    fn fetch_envelopes(&mut self, page_size: &usize, page: &usize) -> Result<Envelopes>;
    fn fetch_envelopes_with(
        &'a mut self,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Envelopes>;
    fn find_msg(&mut self, seq: &str) -> Result<Msg>;
    fn find_raw_msg(&mut self, seq: &str) -> Result<Vec<u8>>;
    fn append_msg(&mut self, mbox: &Mbox, msg: Msg) -> Result<()>;
    fn append_raw_msg_with_flags(&mut self, mbox: &Mbox, msg: &[u8], flags: Flags) -> Result<()>;
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
    mbox: &'a Mbox<'a>,
    sess: Option<ImapSession>,
    /// Holds raw mailboxes fetched by the `imap` crate in order to extend mailboxes lifetime
    /// outside of handlers. Without that, it would be impossible for handlers to return a `Mbox`
    /// struct or a `Mboxes` struct due to the `ZeroCopy` constraint.
    _raw_mboxes_cache: Option<RawMboxes>,
    _raw_msgs_cache: Option<RawEnvelopes>,
}

impl<'a> ImapService<'a> {
    fn sess(&mut self) -> Result<&mut ImapSession> {
        if self.sess.is_none() {
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
            let mut sess = client
                .login(&self.account.imap_login, &self.account.imap_passwd()?)
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

    fn search_new_msgs(&mut self, account: &Account) -> Result<Vec<u32>> {
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
}

impl<'a> ImapServiceInterface<'a> for ImapService<'a> {
    fn fetch_mboxes(&'a mut self) -> Result<Mboxes> {
        let raw_mboxes = self
            .sess()?
            .list(Some(""), Some("*"))
            .context("cannot list mailboxes")?;
        self._raw_mboxes_cache = Some(raw_mboxes);
        Ok(Mboxes::from(self._raw_mboxes_cache.as_ref().unwrap()))
    }

    fn fetch_envelopes(&mut self, page_size: &usize, page: &usize) -> Result<Envelopes> {
        debug!("fetch envelopes");
        debug!("page size: {:?}", page_size);
        debug!("page: {:?}", page);

        let mbox = self.mbox.to_owned();
        let last_seq = self
            .sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?
            .exists as i64;
        debug!("last sequence number: {:?}", last_seq);

        if last_seq == 0 {
            return Ok(Envelopes::default());
        }

        // TODO: add tests, improve error management when empty page
        let range = if *page_size > 0 {
            let cursor = (page * page_size) as i64;
            let begin = 1.max(last_seq - cursor);
            let end = begin - begin.min(*page_size as i64) + 1;
            format!("{}:{}", end, begin)
        } else {
            String::from("1:*")
        };
        debug!("range: {}", range);

        let fetches = self
            .sess()?
            .fetch(&range, "(ENVELOPE FLAGS INTERNALDATE)")
            .context(format!(r#"cannot fetch messages within range "{}""#, range))?;
        self._raw_msgs_cache = Some(fetches);
        Envelopes::try_from(self._raw_msgs_cache.as_ref().unwrap())
    }

    fn fetch_envelopes_with(
        &'a mut self,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Envelopes> {
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
        self._raw_msgs_cache = Some(fetches);
        Envelopes::try_from(self._raw_msgs_cache.as_ref().unwrap())
    }

    /// Find a message by sequence number.
    fn find_msg(&mut self, seq: &str) -> Result<Msg> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox {}", self.mbox.name))?;
        let fetches = self
            .sess()?
            .fetch(seq, "(ENVELOPE FLAGS INTERNALDATE BODY[])")
            .context(r#"cannot fetch messages "{}""#)?;
        let fetch = fetches
            .first()
            .ok_or_else(|| anyhow!(r#"cannot find message "{}"#, seq))?;

        Msg::try_from(fetch)
    }

    fn find_raw_msg(&mut self, seq: &str) -> Result<Vec<u8>> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        let fetches = self
            .sess()?
            .fetch(seq, "BODY[]")
            .context(r#"cannot fetch raw messages "{}""#)?;
        let fetch = fetches
            .first()
            .ok_or_else(|| anyhow!(r#"cannot find raw message "{}"#, seq))?;

        Ok(fetch.body().map(Vec::from).unwrap_or_default())
    }

    fn append_raw_msg_with_flags(&mut self, mbox: &Mbox, msg: &[u8], flags: Flags) -> Result<()> {
        self.sess()?
            .append(&mbox.name, msg)
            .flags(flags.0)
            .finish()
            .context(format!(r#"cannot append message to "{}""#, mbox.name))?;
        Ok(())
    }

    fn append_msg(&mut self, mbox: &Mbox, msg: Msg) -> Result<()> {
        let msg_raw: Vec<u8> = (&msg).try_into()?;
        self.sess()?
            .append(&mbox.name, &msg_raw)
            .flags(msg.flags.0)
            .finish()
            .context(format!(r#"cannot append message to "{}""#, mbox.name))?;
        Ok(())
    }

    fn notify(&mut self, config: &Config, account: &Account, keepalive: u64) -> Result<()> {
        debug!("notify");

        let mbox = self.mbox.to_owned();

        debug!("examine mailbox {:?}", mbox);
        self.sess()?
            .examine(&mbox.name)
            .context(format!("cannot examine mailbox {}", self.mbox.name))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> = self
            .search_new_msgs(account)?
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
                .search_new_msgs(account)?
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

    fn watch(&mut self, account: &Account, keepalive: u64) -> Result<()> {
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

    fn logout(&mut self) -> Result<()> {
        if let Some(ref mut sess) = self.sess {
            debug!("logout from IMAP server");
            sess.logout().context("cannot logout from IMAP server")?;
        }
        Ok(())
    }

    fn add_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox;
        let flags: String = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(seq_range, format!("+FLAGS ({})", flags))
            .context(format!(r#"cannot add flags "{}""#, &flags))?;
        Ok(())
    }

    fn set_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox;
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(seq_range, format!("FLAGS ({})", flags))
            .context(format!(r#"cannot set flags "{}""#, &flags))?;
        Ok(())
    }

    fn remove_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()> {
        let mbox = self.mbox;
        let flags = flags.to_string();
        self.sess()?
            .select(&mbox.name)
            .context(format!(r#"cannot select mailbox "{}""#, self.mbox.name))?;
        self.sess()?
            .store(seq_range, format!("-FLAGS ({})", flags))
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

impl<'a> From<(&'a Account, &'a Mbox<'a>)> for ImapService<'a> {
    fn from((account, mbox): (&'a Account, &'a Mbox)) -> Self {
        Self {
            account,
            mbox,
            sess: None,
            _raw_mboxes_cache: None,
            _raw_msgs_cache: None,
        }
    }
}
