//! # pimdir mailbox
//!
//! The `pimdir mailbox` command family, over the mailboxes the sync
//! engine creates, renames and deletes on the store's behalf and the
//! coverage each is listed with.

pub mod cli;
pub mod create;
pub mod delete;
pub mod list;
pub mod rename;
