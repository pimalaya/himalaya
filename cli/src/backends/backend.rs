//! Backend module.
//!
//! This module exposes the backend trait, which can be used to create
//! custom backend implementations.

use anyhow::Result;
use himalaya_lib::{mbox::Mboxes, msg::Envelopes};

use crate::msg::Msg;

pub trait Backend<'a> {
    fn connect(&mut self) -> Result<()> {
        Ok(())
    }

    fn add_mbox(&mut self, mbox: &str) -> Result<()>;
    fn get_mboxes(&mut self) -> Result<Mboxes>;
    fn del_mbox(&mut self, mbox: &str) -> Result<()>;
    fn get_envelopes(&mut self, mbox: &str, page_size: usize, page: usize) -> Result<Envelopes>;
    fn search_envelopes(
        &mut self,
        mbox: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;
    fn add_msg(&mut self, mbox: &str, msg: &[u8], flags: &str) -> Result<String>;
    fn get_msg(&mut self, mbox: &str, id: &str) -> Result<Msg>;
    fn copy_msg(&mut self, mbox_src: &str, mbox_dst: &str, ids: &str) -> Result<()>;
    fn move_msg(&mut self, mbox_src: &str, mbox_dst: &str, ids: &str) -> Result<()>;
    fn del_msg(&mut self, mbox: &str, ids: &str) -> Result<()>;
    fn add_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;
    fn set_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;
    fn del_flags(&mut self, mbox: &str, ids: &str, flags: &str) -> Result<()>;

    fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }
}
