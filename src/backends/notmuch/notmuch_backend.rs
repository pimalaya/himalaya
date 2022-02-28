use std::{convert::TryInto, fs};

use anyhow::{anyhow, Context, Result};

use crate::{
    backends::{Backend, IdMapper, NotmuchEnvelopes, NotmuchMbox, NotmuchMboxes},
    config::{AccountConfig, NotmuchBackendConfig},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

pub struct NotmuchBackend<'a> {
    account_config: &'a AccountConfig,
    notmuch_config: &'a NotmuchBackendConfig,
    db: notmuch::Database,
}

impl<'a> NotmuchBackend<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        notmuch_config: &'a NotmuchBackendConfig,
    ) -> Result<Self> {
        Ok(Self {
            account_config,
            notmuch_config,
            db: notmuch::Database::open(
                notmuch_config.notmuch_database_dir.clone(),
                notmuch::DatabaseMode::ReadWrite,
            )
            .context(format!(
                "cannot open notmuch database at {:?}",
                notmuch_config.notmuch_database_dir
            ))?,
        })
    }

    fn _search_envelopes(
        &mut self,
        query: &str,
        virt_mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        // Gets envelopes matching the given Notmuch query.
        let query_builder = self
            .db
            .create_query(query)
            .context(format!("cannot create notmuch query from {:?}", query))?;
        let mut envelopes: NotmuchEnvelopes = query_builder
            .search_messages()
            .context(format!(
                "cannot find notmuch envelopes from query {:?}",
                query
            ))?
            .try_into()
            .context(format!(
                "cannot parse notmuch envelopes from query {:?}",
                query
            ))?;

        // Calculates pagination boundaries.
        let page_begin = page * page_size;
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot find notmuch envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);

        // Sorts envelopes by most recent date.
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        // Applies pagination boundaries.
        envelopes.0 = envelopes[page_begin..page_end].to_owned();

        // Appends id <=> hash entries to the id mapper cache file.
        let short_hash_len = {
            let mut mapper = IdMapper::new(&self.notmuch_config.notmuch_database_dir)?;
            let entries = envelopes
                .iter()
                .map(|env| (env.hash.to_owned(), env.id.to_owned()))
                .collect();
            mapper.append(entries)?
        };

        // Shorten envelopes hash.
        envelopes
            .iter_mut()
            .for_each(|env| env.hash = env.hash[0..short_hash_len].to_owned());

        Ok(Box::new(envelopes))
    }
}

impl<'a> Backend<'a> for NotmuchBackend<'a> {
    fn add_mbox(&mut self, _mbox: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        let mut mboxes: Vec<_> = self
            .account_config
            .mailboxes
            .iter()
            .map(|(k, v)| NotmuchMbox::new(k, v))
            .collect();
        mboxes.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());
        Ok(Box::new(NotmuchMboxes(mboxes)))
    }

    fn del_mbox(&mut self, _mbox: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_envelopes(
        &mut self,
        virt_mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let query = self
            .account_config
            .mailboxes
            .get(virt_mbox)
            .map(|s| s.as_str())
            .unwrap_or("all");
        self._search_envelopes(query, virt_mbox, page_size, page)
    }

    fn search_envelopes(
        &mut self,
        virt_mbox: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        self._search_envelopes(query, virt_mbox, page_size, page)
    }

    fn add_msg(&mut self, _mbox: &str, _msg: &[u8], _flags: &str) -> Result<Box<dyn ToString>> {
        unimplemented!();
    }

    fn get_msg(&mut self, _mbox: &str, short_hash: &str) -> Result<Msg> {
        let id = IdMapper::new(&self.notmuch_config.notmuch_database_dir)?
            .find(short_hash)
            .context(format!(
                "cannot get notmuch message from short hash {:?}",
                short_hash
            ))?;
        let msg_filepath = self
            .db
            .find_message(&id)
            .context(format!("cannot find notmuch message {:?}", id))?
            .ok_or_else(|| anyhow!("cannot find notmuch message {:?}", id))?
            .filename()
            .to_owned();
        let raw_msg = fs::read(&msg_filepath)
            .context(format!("cannot read message from file {:?}", msg_filepath))?;
        let msg = Msg::from_parsed_mail(mailparse::parse_mail(&raw_msg)?, &self.account_config)?;
        Ok(msg)
    }

    fn copy_msg(&mut self, _mbox_src: &str, _mbox_dst: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn move_msg(&mut self, _mbox_src: &str, _mbox_dst: &str, _id: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_msg(&mut self, _mbox: &str, short_hash: &str) -> Result<()> {
        let id = IdMapper::new(&self.notmuch_config.notmuch_database_dir)?
            .find(short_hash)
            .context(format!(
                "cannot get notmuch message from short hash {:?}",
                short_hash
            ))?;
        let msg_filepath = self
            .db
            .find_message(&id)
            .context(format!("cannot find notmuch message {:?}", id))?
            .ok_or_else(|| anyhow!("cannot find notmuch message {:?}", id))?
            .filename()
            .to_owned();
        self.db
            .remove_message(msg_filepath)
            .context(format!("cannot delete notmuch message {:?}", id))
    }

    fn add_flags(&mut self, _mbox: &str, _id: &str, _flags_str: &str) -> Result<()> {
        unimplemented!();
    }

    fn set_flags(&mut self, _mbox: &str, _id: &str, _flags_str: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_flags(&mut self, _mbox: &str, _id: &str, _flags_str: &str) -> Result<()> {
        unimplemented!();
    }
}
