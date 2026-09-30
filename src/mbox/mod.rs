//! # mbox
//!
//! The `mbox` command family, covering what maps onto mbox files, plus
//! the adapter serving the shared commands over mbox.

pub mod arg;
pub mod backend;
pub mod cache;
pub mod cli;
pub mod client;
pub mod create;
pub mod delete;
pub mod flag;
pub mod list;
pub mod message;
pub mod rename;
