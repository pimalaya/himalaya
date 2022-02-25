use std::convert::TryInto;

use anyhow::{Context, Result};

use crate::{
    backends::Backend,
    config::{AccountConfig, NotmuchBackendConfig},
    mbox::Mboxes,
    msg::{Envelopes, Msg},
};

use super::NotmuchEnvelopes;

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
    fn add_mbox(&mut self, mdir: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>> {
        unimplemented!();
    }

    fn del_mbox(&mut self, mdir: &str) -> Result<()> {
        unimplemented!();
    }

    fn get_envelopes(
        &mut self,
        mdir: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        unimplemented!();
    }

    fn find_envelopes(
        &mut self,
        _mdir: &str,
        query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Box<dyn Envelopes>> {
        let query_builder = self
            .db
            .create_query(query)
            .context("cannot create notmuch query")?;
        let msgs: NotmuchEnvelopes = query_builder
            .search_messages()
            .context(format!(
                "cannot find notmuch envelopes with query {:?}",
                query
            ))?
            .try_into()?;
        Ok(Box::new(msgs))
    }

    fn add_msg(&mut self, mdir: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>> {
        unimplemented!();
    }

    fn get_msg(&mut self, mdir: &str, id: &str) -> Result<Msg> {
        unimplemented!();
    }

    fn copy_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        unimplemented!();
    }

    fn move_msg(&mut self, mdir_src: &str, mdir_dst: &str, id: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_msg(&mut self, mdir: &str, id: &str) -> Result<()> {
        unimplemented!();
    }

    fn add_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        unimplemented!();
    }

    fn set_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        unimplemented!();
    }

    fn del_flags(&mut self, mdir: &str, id: &str, flags_str: &str) -> Result<()> {
        unimplemented!();
    }
}
