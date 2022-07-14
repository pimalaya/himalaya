//! Backend module.
//!
//! This module exposes the backend trait, which can be used to create
//! custom backend implementations.

use std::result;

use thiserror::Error;

use crate::{
    account,
    mbox::Mboxes,
    msg::{self, Envelopes, Msg},
};

use super::id_mapper;

#[cfg(feature = "maildir-backend")]
use super::MaildirError;

#[cfg(feature = "notmuch-backend")]
use super::NotmuchError;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ImapError(#[from] super::imap::Error),

    #[error(transparent)]
    AccountError(#[from] account::AccountError),

    #[error(transparent)]
    MsgError(#[from] msg::Error),

    #[error(transparent)]
    IdMapperError(#[from] id_mapper::Error),

    #[cfg(feature = "maildir-backend")]
    #[error(transparent)]
    MaildirError(#[from] MaildirError),

    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchError(#[from] NotmuchError),
}

pub type Result<T> = result::Result<T, Error>;

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
