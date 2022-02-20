//! Backend module.
//!
//! This module exposes the backend trait, which can be used to create
//! custom backend implementations.

use anyhow::Result;

use crate::{config::AccountConfig, domain::Msg, mbox::Mboxes, msg::Envelopes};

pub trait Backend<'a> {
    fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_mboxes(&mut self) -> Result<Box<dyn Mboxes>>;
    fn get_envelopes(
        &mut self,
        mbox: &str,
        filter: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Box<dyn Envelopes>>;
    fn add_msg(&mut self, mbox: &str, msg: &[u8], flags: &str) -> Result<Box<dyn ToString>>;
    fn get_msg(&mut self, mbox: &str, id: &str, config: &AccountConfig) -> Result<Msg>;
    fn copy_msg(
        &mut self,
        mbox_src: &str,
        mbox_dst: &str,
        ids: &str,
        config: &AccountConfig,
    ) -> Result<()>;
    fn move_msg(
        &mut self,
        mbox_src: &str,
        mbox_dst: &str,
        ids: &str,
        config: &AccountConfig,
    ) -> Result<()>;
    fn del_msg(&mut self, mbox: &str, ids: &str) -> Result<()>;
    fn add_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;
    fn set_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;
    fn del_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;

    fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }
}
