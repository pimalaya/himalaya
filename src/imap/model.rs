use error_chain::error_chain;
use imap;
use log::{debug, trace};
use native_tls::{self, TlsConnector, TlsStream};
use std::{collections::HashSet, iter::FromIterator, net::TcpStream};

use crate::config::model::Account;
use crate::ctx::Ctx;
use imap::types::Flag;
use crate::msg::mail::Mail;

error_chain! {
    links {
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
    }
}

#[derive(Debug)]
pub struct ImapConnector<'a> {
    pub account: &'a Account,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl<'a> ImapConnector<'a> {
    pub fn new(account: &'a Account) -> Result<Self> {
        debug!("create TLS builder");
        let insecure = account.imap_insecure();
        let tls = TlsConnector::builder()
            .danger_accept_invalid_certs(insecure)
            .danger_accept_invalid_hostnames(insecure)
            .build()
            .chain_err(|| "Could not create TLS connector")?;

        debug!("create client");
        let client = if account.imap_starttls() {
            imap::connect_starttls(account.imap_addr(), &account.imap_host, &tls)
                .chain_err(|| "Could not connect using STARTTLS")
        } else {
            imap::connect(account.imap_addr(), &account.imap_host, &tls)
                .chain_err(|| "Could not connect using TLS")
        }?;

        debug!("create session");
        let sess = client
            .login(&account.imap_login, &account.imap_passwd()?)
            .map_err(|res| res.0)
            .chain_err(|| "Could not login to IMAP server")?;

        Ok(Self { account, sess })
    }

    pub fn logout(&mut self) {
        debug!("logout");
        match self.sess.logout() {
            _ => (),
        }
    }

    pub fn set_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("FLAGS ({})", flags))
            .chain_err(|| format!("Could not set flags `{}`", &flags))?;

        Ok(())
    }

    pub fn add_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("+FLAGS ({})", flags))
            .chain_err(|| format!("Could not add flags `{}`", &flags))?;

        Ok(())
    }

    pub fn remove_flags(&mut self, mbox: &str, uid_seq: &str, flags: &str) -> Result<()> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;

        self.sess
            .uid_store(uid_seq, format!("-FLAGS ({})", flags))
            .chain_err(|| format!("Could not remove flags `{}`", &flags))?;

        Ok(())
    }

    fn search_new_msgs(&mut self) -> Result<Vec<u32>> {
        let uids: Vec<u32> = self
            .sess
            .uid_search("NEW")
            .chain_err(|| "Could not search new messages")?
            .into_iter()
            .collect();
        debug!("found {} new messages", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }

    pub fn notify(&mut self, ctx: &Ctx, keepalive: u64) -> Result<()> {
        debug!("examine mailbox: {}", &ctx.mbox);
        self.sess
            .examine(&ctx.mbox)
            .chain_err(|| format!("Could not examine mailbox `{}`", &ctx.mbox))?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> =
            HashSet::from_iter(self.search_new_msgs()?.iter().cloned());
        trace!("messages hashset: {:?}", msgs_set);

        loop {
            debug!("begin loop");
            self.sess
                .idle()
                .and_then(|mut idle| {
                    idle.set_keepalive(std::time::Duration::new(keepalive, 0));
                    idle.wait_keepalive_while(|res| {
                        // TODO: handle response
                        trace!("idle response: {:?}", res);
                        false
                    })
                })
                .chain_err(|| "Could not start the idle mode")?;

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
                    .sess
                    .uid_fetch(uids, "(ENVELOPE)")
                    .chain_err(|| "Could not fetch new messages enveloppe")?;

                for fetch in fetches.iter() {
                    let msg = Mail::from(fetch);
                    let uid = fetch.uid.ok_or_else(|| {
                        format!("Could not retrieve message {}'s UID", fetch.message)
                    })?;

                    ctx.config.run_notify_cmd(&msg.envelope.get_subject(), &msg.envelope.get_sender())?;

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

    pub fn watch(&mut self, ctx: &Ctx, keepalive: u64) -> Result<()> {
        debug!("examine mailbox: {}", &ctx.mbox);
        self.sess
            .examine(&ctx.mbox)
            .chain_err(|| format!("Could not examine mailbox `{}`", &ctx.mbox))?;

        loop {
            debug!("begin loop");
            self.sess
                .idle()
                .and_then(|mut idle| {
                    idle.set_keepalive(std::time::Duration::new(keepalive, 0));
                    idle.wait_keepalive_while(|res| {
                        // TODO: handle response
                        trace!("idle response: {:?}", res);
                        false
                    })
                })
                .chain_err(|| "Could not start the idle mode")?;
            ctx.config.exec_watch_cmds(&ctx.account)?;
            debug!("end loop");
        }
    }

    pub fn list_mboxes(&mut self) -> Result<imap::types::ZeroCopy<Vec<imap::types::Name>>> {
        let names = self
            .sess
            .list(Some(""), Some("*"))
            .chain_err(|| "Could not list mailboxes")?;

        Ok(names)
    }

    pub fn list_msgs(
        &mut self,
        mbox: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Option<imap::types::ZeroCopy<Vec<imap::types::Fetch>>>> {
        let last_seq = self
            .sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?
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
            .sess
            .fetch(range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .chain_err(|| "Could not fetch messages")?;

        Ok(Some(fetches))
    }

    pub fn search_msgs(
        &mut self,
        mbox: &str,
        query: &str,
        page_size: &usize,
        page: &usize,
    ) -> Result<Option<imap::types::ZeroCopy<Vec<imap::types::Fetch>>>> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;

        let begin = page * page_size;
        let end = begin + (page_size - 1);
        let uids: Vec<String> = self
            .sess
            .search(query)
            .chain_err(|| format!("Could not search in `{}` with query `{}`", mbox, query))?
            .iter()
            .map(|seq| seq.to_string())
            .collect();

        if uids.is_empty() {
            return Ok(None);
        }

        let range = uids[begin..end.min(uids.len())].join(",");
        let fetches = self
            .sess
            .fetch(&range, "(UID FLAGS ENVELOPE INTERNALDATE)")
            .chain_err(|| format!("Could not fetch range `{}`", &range))?;

        Ok(Some(fetches))
    }

    // pub fn read_msg(&mut self, mbox: &str, uid: &str) -> Result<Vec<u8>> {
    //     self.sess
    //         .select(mbox)
    //         .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;
    //
    //     match self
    //         .sess
    //         .uid_fetch(uid, "(FLAGS BODY[])")
    //         .chain_err(|| "Could not fetch bodies")?
    //         .first()
    //     {
    //         None => Err(format!("Could not find message `{}`", uid).into()),
    //         Some(fetch) => Ok(fetch.body().unwrap_or(&[]).to_vec()),
    //     }
    // }

    pub fn read_msg(&mut self, mbox: &str, uid: &str) -> Result<Mail> {
        self.sess
            .select(mbox)
            .chain_err(|| format!("Could not select mailbox `{}`", mbox))?;

        match self
            .sess
            .uid_fetch(uid, "(FLAGS BODY[])")
            .chain_err(|| "Could not fetch bodies")?
            .first()
        {
            None => Err(format!("Could not find message `{}`", uid).into()),
            Some(fetch) => Ok(Mail::from(fetch)),
        }
    }

    pub fn append_msg(&mut self, mbox: &str, msg: &[u8], flags: Vec<Flag>) -> Result<()> {
        self.sess
            .append(mbox, msg)
            .flags(flags)
            .finish()
            .chain_err(|| format!("Could not append message to `{}`", mbox))?;

        Ok(())
    }

    pub fn expunge(&mut self, mbox: &str) -> Result<()> {
        self.sess
            .expunge()
            .chain_err(|| format!("Could not expunge `{}`", mbox))?;

        Ok(())
    }
}
