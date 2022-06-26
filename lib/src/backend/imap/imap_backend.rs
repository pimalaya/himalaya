//! IMAP backend module.
//!
//! This module contains the definition of the IMAP backend.

use imap::types::NameAttribute;
use log::{debug, log_enabled, trace, Level};
use native_tls::{TlsConnector, TlsStream};
use std::{collections::HashSet, convert::TryInto, net::TcpStream, thread};

use crate::{
    account::{Account, ImapBackendConfig},
    backend::{
        backend::Result, from_imap_fetch, from_imap_fetches,
        imap::msg_sort_criterion::SortCriteria, imap::Error, into_imap_flags, Backend,
    },
    mbox::{Mbox, Mboxes},
    msg::{Envelopes, Flags, Msg},
    process,
};

type ImapSess = imap::Session<TlsStream<TcpStream>>;

pub struct ImapBackend<'a> {
    account_config: &'a Account,
    imap_config: &'a ImapBackendConfig,
    sess: Option<ImapSess>,
}

impl<'a> ImapBackend<'a> {
    pub fn new(account_config: &'a Account, imap_config: &'a ImapBackendConfig) -> Self {
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
                .map_err(Error::CreateTlsConnectorError)?;

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
                .map_err(Error::ConnectImapServerError)?;

            debug!("create session");
            debug!("login: {}", self.imap_config.imap_login);
            debug!("passwd cmd: {}", self.imap_config.imap_passwd_cmd);
            let mut sess = client
                .login(
                    &self.imap_config.imap_login,
                    &self.imap_config.imap_passwd()?,
                )
                .map_err(|res| Error::LoginImapServerError(res.0))?;
            sess.debug = log_enabled!(Level::Trace);
            self.sess = Some(sess);
        }

        let sess = match self.sess {
            Some(ref mut sess) => Ok(sess),
            None => Err(Error::GetSessionError),
        }?;

        Ok(sess)
    }

    fn search_new_msgs(&mut self, query: &str) -> Result<Vec<u32>> {
        let uids: Vec<u32> = self
            .sess()?
            .uid_search(query)
            .map_err(Error::SearchNewMsgsError)?
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
            .map_err(|err| Error::ExamineMboxError(err, mbox.to_owned()))?;

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
                .map_err(Error::StartIdleModeError)?;

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
                    .map_err(Error::FetchNewMsgsEnvelopeError)?;

                for fetch in fetches.iter() {
                    let msg = from_imap_fetch(fetch)?;
                    let uid = fetch.uid.ok_or_else(|| Error::GetUidError(fetch.message))?;

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
            .map_err(|err| Error::ExamineMboxError(err, mbox.to_owned()))?;

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
                .map_err(Error::StartIdleModeError)?;

            let cmds = self.account_config.watch_cmds.clone();
            thread::spawn(move || {
                debug!("batch execution of {} cmd(s)", cmds.len());
                cmds.iter().for_each(|cmd| {
                    debug!("running command {:?}…", cmd);
                    let res = process::run(cmd);
                    debug!("{:?}", res);
                })
            });

            debug!("end loop");
        }
    }
}

impl<'a> Backend<'a> for ImapBackend<'a> {
    fn add_mbox(&mut self, mbox: &str) -> Result<()> {
        trace!(">> add mailbox");

        self.sess()?
            .create(mbox)
            .map_err(|err| Error::CreateMboxError(err, mbox.to_owned()))?;

        trace!("<< add mailbox");
        Ok(())
    }

    fn get_mboxes(&mut self) -> Result<Mboxes> {
        trace!(">> get imap mailboxes");

        let imap_mboxes = self
            .sess()?
            .list(Some(""), Some("*"))
            .map_err(Error::ListMboxesError)?;
        let mboxes = Mboxes {
            mboxes: imap_mboxes
                .iter()
                .map(|imap_mbox| Mbox {
                    delim: imap_mbox.delimiter().unwrap_or_default().into(),
                    name: imap_mbox.name().into(),
                    desc: imap_mbox
                        .attributes()
                        .iter()
                        .map(|attr| match attr {
                            NameAttribute::Marked => "Marked",
                            NameAttribute::Unmarked => "Unmarked",
                            NameAttribute::NoSelect => "NoSelect",
                            NameAttribute::NoInferiors => "NoInferiors",
                            NameAttribute::Custom(custom) => custom.trim_start_matches('\\'),
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                })
                .collect(),
        };

        trace!("imap mailboxes: {:?}", mboxes);
        trace!("<< get imap mailboxes");
        Ok(mboxes)
    }

    fn del_mbox(&mut self, mbox: &str) -> Result<()> {
        trace!(">> delete imap mailbox");

        self.sess()?
            .delete(mbox)
            .map_err(|err| Error::DeleteMboxError(err, mbox.to_owned()))?;

        trace!("<< delete imap mailbox");
        Ok(())
    }

    fn get_envelopes(&mut self, mbox: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        let last_seq = self
            .sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?
            .exists as usize;
        debug!("last sequence number: {:?}", last_seq);
        if last_seq == 0 {
            return Ok(Envelopes::default());
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
            .map_err(|err| Error::FetchMsgsByRangeError(err, range.to_owned()))?;

        let envelopes = from_imap_fetches(fetches)?;
        Ok(envelopes)
    }

    fn search_envelopes(
        &mut self,
        mbox: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        let last_seq = self
            .sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?
            .exists;
        debug!("last sequence number: {:?}", last_seq);
        if last_seq == 0 {
            return Ok(Envelopes::default());
        }

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let seqs: Vec<String> = if sort.is_empty() {
            self.sess()?
                .search(query)
                .map_err(|err| Error::SearchMsgsError(err, mbox.to_owned(), query.to_owned()))?
                .iter()
                .map(|seq| seq.to_string())
                .collect()
        } else {
            let sort: SortCriteria = sort.try_into()?;
            let charset = imap::extensions::sort::SortCharset::Utf8;
            self.sess()?
                .sort(&sort, charset, query)
                .map_err(|err| Error::SortMsgsError(err, mbox.to_owned(), query.to_owned()))?
                .iter()
                .map(|seq| seq.to_string())
                .collect()
        };
        if seqs.is_empty() {
            return Ok(Envelopes::default());
        }

        let range = seqs[begin..end.min(seqs.len())].join(",");
        let fetches = self
            .sess()?
            .fetch(&range, "(ENVELOPE FLAGS INTERNALDATE)")
            .map_err(|err| Error::FetchMsgsByRangeError(err, range.to_owned()))?;

        let envelopes = from_imap_fetches(fetches)?;
        Ok(envelopes)
    }

    fn add_msg(&mut self, mbox: &str, msg: &[u8], flags: &str) -> Result<String> {
        let flags: Flags = flags.into();
        self.sess()?
            .append(mbox, msg)
            .flags(into_imap_flags(&flags))
            .finish()
            .map_err(|err| Error::AppendMsgError(err, mbox.to_owned()))?;
        let last_seq = self
            .sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?
            .exists;
        Ok(last_seq.to_string())
    }

    fn get_msg(&mut self, mbox: &str, seq: &str) -> Result<Msg> {
        self.sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?;
        let fetches = self
            .sess()?
            .fetch(seq, "(FLAGS INTERNALDATE BODY[])")
            .map_err(|err| Error::FetchMsgsBySeqError(err, seq.to_owned()))?;
        let fetch = fetches
            .first()
            .ok_or_else(|| Error::FindMsgError(seq.to_owned()))?;
        let msg_raw = fetch.body().unwrap_or_default().to_owned();
        let mut msg = Msg::from_parsed_mail(
            mailparse::parse_mail(&msg_raw)
                .map_err(|err| Error::ParseMsgError(err, seq.to_owned()))?,
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
        let flags: Flags = flags.into();
        self.sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?;
        self.sess()?
            .store(seq_range, format!("+FLAGS ({})", flags))
            .map_err(|err| Error::AddFlagsError(err, flags.to_owned(), seq_range.to_owned()))?;
        self.sess()?
            .expunge()
            .map_err(|err| Error::ExpungeError(err, mbox.to_owned()))?;
        Ok(())
    }

    fn set_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: Flags = flags.into();
        self.sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?;
        self.sess()?
            .store(seq_range, format!("FLAGS ({})", flags))
            .map_err(|err| Error::SetFlagsError(err, flags.to_owned(), seq_range.to_owned()))?;
        Ok(())
    }

    fn del_flags(&mut self, mbox: &str, seq_range: &str, flags: &str) -> Result<()> {
        let flags: Flags = flags.into();
        self.sess()?
            .select(mbox)
            .map_err(|err| Error::SelectMboxError(err, mbox.to_owned()))?;
        self.sess()?
            .store(seq_range, format!("-FLAGS ({})", flags))
            .map_err(|err| Error::DelFlagsError(err, flags.to_owned(), seq_range.to_owned()))?;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<()> {
        trace!(">> imap logout");

        if let Some(ref mut sess) = self.sess {
            debug!("logout from imap server");
            sess.logout().map_err(Error::LogoutError)?;
        } else {
            debug!("no session found");
        }

        trace!("<< imap logout");
        Ok(())
    }
}
