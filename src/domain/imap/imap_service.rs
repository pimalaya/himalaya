//! Module related to IMAP servicing.
//!
//! This module exposes a service that can interact with IMAP servers.

use anyhow::{anyhow, Context, Error, Result};
use log::{debug, log_enabled, trace, Level};
use native_tls::{TlsConnector, TlsStream};
use std::{collections::HashSet, convert::TryFrom, net::TcpStream, thread};

use crate::{
    config::{AccountConfig, ImapBackendConfig},
    domain::{Envelope, Envelopes, Flags, Mbox, Mboxes, Msg, RawEnvelopes, RawMboxes},
    output::run_cmd,
};

type ImapSession = imap::Session<TlsStream<TcpStream>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortCriterionKind {
    Arrival,
    Cc,
    Date,
    From,
    Size,
    Subject,
    To,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortCriterionOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortCriterion {
    kind: SortCriterionKind,
    order: SortCriterionOrder,
}

impl TryFrom<&str> for SortCriterion {
    type Error = Error;

    fn try_from(criterion: &str) -> Result<Self, Self::Error> {
        match criterion {
            "arrival:asc" | "arrival" => Ok(Self {
                kind: SortCriterionKind::Arrival,
                order: SortCriterionOrder::Asc,
            }),
            "arrival:desc" => Ok(Self {
                kind: SortCriterionKind::Arrival,
                order: SortCriterionOrder::Desc,
            }),
            "cc:asc" | "cc" => Ok(Self {
                kind: SortCriterionKind::Cc,
                order: SortCriterionOrder::Asc,
            }),
            "cc:desc" => Ok(Self {
                kind: SortCriterionKind::Cc,
                order: SortCriterionOrder::Desc,
            }),
            "date:asc" | "date" => Ok(Self {
                kind: SortCriterionKind::Date,
                order: SortCriterionOrder::Asc,
            }),
            "date:desc" => Ok(Self {
                kind: SortCriterionKind::Date,
                order: SortCriterionOrder::Desc,
            }),
            "from:asc" | "from" => Ok(Self {
                kind: SortCriterionKind::From,
                order: SortCriterionOrder::Asc,
            }),
            "from:desc" => Ok(Self {
                kind: SortCriterionKind::From,
                order: SortCriterionOrder::Desc,
            }),
            "size:asc" | "size" => Ok(Self {
                kind: SortCriterionKind::Size,
                order: SortCriterionOrder::Asc,
            }),
            "size:desc" => Ok(Self {
                kind: SortCriterionKind::Size,
                order: SortCriterionOrder::Desc,
            }),
            "subject:asc" | "subject" => Ok(Self {
                kind: SortCriterionKind::Subject,
                order: SortCriterionOrder::Asc,
            }),
            "subject:desc" => Ok(Self {
                kind: SortCriterionKind::Subject,
                order: SortCriterionOrder::Desc,
            }),
            "to:asc" | "to" => Ok(Self {
                kind: SortCriterionKind::To,
                order: SortCriterionOrder::Asc,
            }),
            "to:desc" => Ok(Self {
                kind: SortCriterionKind::To,
                order: SortCriterionOrder::Desc,
            }),
            _ => Err(anyhow!("cannot parse sort criterion {:?}", criterion)),
        }
    }
}

impl<'a> Into<imap::extensions::sort::SortCriterion<'a>> for &'a SortCriterion {
    fn into(self) -> imap::extensions::sort::SortCriterion<'a> {
        let criterion = match self.kind {
            SortCriterionKind::Arrival => &imap::extensions::sort::SortCriterion::Arrival,
            SortCriterionKind::Cc => &imap::extensions::sort::SortCriterion::Cc,
            SortCriterionKind::Date => &imap::extensions::sort::SortCriterion::Date,
            SortCriterionKind::From => &imap::extensions::sort::SortCriterion::From,
            SortCriterionKind::Size => &imap::extensions::sort::SortCriterion::Size,
            SortCriterionKind::Subject => &imap::extensions::sort::SortCriterion::Subject,
            SortCriterionKind::To => &imap::extensions::sort::SortCriterion::To,
        };
        match self.order {
            SortCriterionOrder::Asc => *criterion,
            SortCriterionOrder::Desc => imap::extensions::sort::SortCriterion::Reverse(criterion),
        }
    }
}

pub trait BackendService<'a> {
    fn connect(&mut self) -> Result<()>;
    fn get_mboxes(&mut self) -> Result<Mboxes>;
    fn get_envelopes(
        &mut self,
        sort: &[SortCriterion],
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Envelopes>;
    fn get_msg(&mut self, account: &AccountConfig, seq: &str) -> Result<Msg>;
    fn add_msg(&mut self, mbox: &Mbox, account: &AccountConfig, msg: Msg) -> Result<()>;
    fn add_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
    fn set_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
    fn del_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()>;
    fn disconnect(&mut self) -> Result<()>;

    fn find_raw_msg(&mut self, seq: &str) -> Result<Vec<u8>>;
    fn append_raw_msg_with_flags(&mut self, mbox: &Mbox, msg: &[u8], flags: Flags) -> Result<()>;
    fn expunge(&mut self) -> Result<()>;
}

pub struct ImapService<'a> {
    backend_config: &'a ImapBackendConfig,
    mbox: &'a Mbox<'a>,
    sess: Option<ImapSession>,
    /// Holds raw mailboxes fetched by the `imap` crate in order to extend mailboxes lifetime
    /// outside of handlers. Without that, it would be impossible for handlers to return a `Mbox`
    /// struct or a `Mboxes` struct due to the `ZeroCopy` constraint.
    _raw_mboxes_cache: Option<RawMboxes>,
    _raw_msgs_cache: Option<RawEnvelopes>,
}

impl<'a> ImapService<'a> {
    pub fn from_config_and_mbox(backend_config: &'a ImapBackendConfig, mbox: &'a Mbox) -> Self {
        Self {
            backend_config,
            mbox,
            sess: None,
            _raw_mboxes_cache: None,
            _raw_msgs_cache: None,
        }
    }

    fn sess(&mut self) -> Result<&mut ImapSession> {
        if self.sess.is_none() {
            debug!("create TLS builder");
            debug!("insecure: {}", self.backend_config.imap_insecure);
            let builder = TlsConnector::builder()
                .danger_accept_invalid_certs(self.backend_config.imap_insecure)
                .danger_accept_invalid_hostnames(self.backend_config.imap_insecure)
                .build()
                .context("cannot create TLS connector")?;

            debug!("create client");
            debug!("host: {}", self.backend_config.imap_host);
            debug!("port: {}", self.backend_config.imap_port);
            debug!("starttls: {}", self.backend_config.imap_starttls);
            let mut client_builder = imap::ClientBuilder::new(
                &self.backend_config.imap_host,
                self.backend_config.imap_port,
            );
            if self.backend_config.imap_starttls {
                client_builder.starttls();
            }
            let client = client_builder
                .connect(|domain, tcp| Ok(TlsConnector::connect(&builder, domain, tcp)?))
                .context("cannot connect to IMAP server")?;

            debug!("create session");
            debug!("login: {}", self.backend_config.imap_login);
            debug!("passwd cmd: {}", self.backend_config.imap_passwd_cmd);
            let mut sess = client
                .login(
                    &self.backend_config.imap_login,
                    &self.backend_config.imap_passwd()?,
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

    pub fn notify(&mut self, config: &AccountConfig, keepalive: u64) -> Result<()> {
        debug!("notify");

        let mbox = self.mbox.to_owned();

        debug!("examine mailbox {:?}", mbox);
        self.sess()?
            .examine(&mbox.name)
            .context(format!("cannot examine mailbox {}", self.mbox.name))?;

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

    pub fn watch(&mut self, account: &AccountConfig, keepalive: u64) -> Result<()> {
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
}

impl<'a> BackendService<'a> for ImapService<'a> {
    fn connect(&mut self) -> Result<()> {
        Ok(())
    }

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
        sort: &[SortCriterion],
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Envelopes> {
        let mbox = self.mbox.to_owned();
        self.sess()?
            .select(&mbox.name)
            .context(format!("cannot select mailbox {:?}", self.mbox.name))?;

        let sort = sort
            .iter()
            .map(|criterion| criterion.into())
            .collect::<Vec<_>>();
        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let seqs: Vec<String> = self
            .sess()?
            .sort(&sort, imap::extensions::sort::SortCharset::Utf8, query)
            .context(format!(
                "cannot search in {:?} with query {:?}",
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
            .context(format!("cannot fetch messages within range {:?}", range))?;
        self._raw_msgs_cache = Some(fetches);
        Envelopes::try_from(self._raw_msgs_cache.as_ref().unwrap())
    }

    /// Find a message by sequence number.
    fn get_msg(&mut self, account: &AccountConfig, seq: &str) -> Result<Msg> {
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

        Msg::try_from((account, fetch))
    }

    fn add_msg(&mut self, mbox: &Mbox, account: &AccountConfig, msg: Msg) -> Result<()> {
        let msg_raw = msg.into_sendable_msg(account)?.formatted();
        self.sess()?
            .append(&mbox.name, &msg_raw)
            .flags(msg.flags.0)
            .finish()
            .context(format!(r#"cannot append message to "{}""#, mbox.name))?;
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

    fn del_flags(&mut self, seq_range: &str, flags: &Flags) -> Result<()> {
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

    fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut sess) = self.sess {
            debug!("logout from IMAP server");
            sess.logout().context("cannot logout from IMAP server")?;
        }
        Ok(())
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

    fn expunge(&mut self) -> Result<()> {
        self.sess()?
            .expunge()
            .context(format!(r#"cannot expunge mailbox "{}""#, self.mbox.name))?;
        Ok(())
    }
}
