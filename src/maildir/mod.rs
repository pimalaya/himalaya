//! # Maildir
//!
//! The `maildir` command family, covering what maps onto the on-disk
//! layout, plus the adapter serving the shared commands over Maildir.

pub mod arg;
pub mod backend;
pub mod cli;
pub mod client;
pub mod create;
pub mod delete;
pub mod flag;
pub mod list;
pub mod message;
pub mod rename;
