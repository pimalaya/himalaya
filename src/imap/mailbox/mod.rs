//! # IMAP mailbox
//!
//! The IMAP commands over a mailbox: its lifecycle, its subscription, its
//! selection and its status.

pub mod arg;
pub mod close;
pub mod create;
pub mod delete;
pub mod expunge;
pub mod list;
pub mod rename;
pub mod select;
pub mod status;
pub mod subscribe;
pub mod unselect;
pub mod unsubscribe;
