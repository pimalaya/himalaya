use std::{convert::TryInto, fs};

use anyhow::{anyhow, Context, Result};

use crate::{
    backends::{Backend, NotmuchEnvelopes},
    config::{AccountConfig, NotmuchBackendConfig},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

pub struct NotmuchBackend<'a> {
    account_config: &'a AccountConfig,
    db: notmuch::Database,
}

impl<'a> NotmuchBackend<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        notmuch_config: &'a NotmuchBackendConfig,
    ) -> Result<Self> {
        Ok(Self {
            account_config,
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
}

impl<'a> Backend<'a> for NotmuchBackend<'a> {
    fn add_mbox(&mut self, _mbox: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        unimplemented!();
    }

    fn del_mbox(&mut self, _mbox: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_envelopes(
        &mut self,
        mbox: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let query = self
            .account_config
            .mailboxes
            .get(mbox)
            .map(|s| s.as_str())
            .unwrap_or("all");
        let query_builder = self
            .db
            .create_query(query)
            .context("cannot create notmuch query")?;
        let mut envelopes: NotmuchEnvelopes = query_builder
            .search_messages()
            .context(format!(
                "cannot find notmuch envelopes with query {:?}",
                query
            ))?
            .try_into()?;
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        let page_begin = page * page_size;
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot find notmuch envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        envelopes.0 = envelopes[page_begin..page_end].to_owned();
        Ok(Box::new(envelopes))
    }

    fn find_envelopes(
        &mut self,
        _mbox: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let query_builder = self
            .db
            .create_query(query)
            .context("cannot create notmuch query")?;
        let mut envelopes: NotmuchEnvelopes = query_builder
            .search_messages()
            .context(format!(
                "cannot find notmuch envelopes with query {:?}",
                query
            ))?
            .try_into()?;
        // TODO: use sort from parameters instead
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        let page_begin = page * page_size;
        if page_begin > envelopes.len() {
            return Err(anyhow!(format!(
                "cannot find notmuch envelopes at page {:?} (out of bounds)",
                page_begin + 1,
            )));
        }
        let page_end = envelopes.len().min(page_begin + page_size);
        envelopes.0 = envelopes[page_begin..page_end].to_owned();
        Ok(Box::new(envelopes))
    }

    fn add_msg(&mut self, _mbox: &str, _msg: &[u8], _flags: &str) -> Result<Box<dyn ToString>> {
        unimplemented!();
    }

    fn get_msg(&mut self, _mbox: &str, id: &str) -> Result<Msg> {
        let msg_filepath = self
            .db
            .find_message(id)
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

    fn del_msg(&mut self, _mbox: &str, _id: &str) -> Result<()> {
        unimplemented!();
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
