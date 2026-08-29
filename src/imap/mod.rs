//! # IMAP
//!
//! The `imap` command family, mirroring the flat RFC 3501 command list,
//! plus the adapter serving the shared commands over IMAP.

pub mod backend;
pub mod cli;
pub mod client;
pub mod envelope;
pub mod fetch;
pub mod flag;
pub mod id;
pub mod mailbox;
pub mod message;
pub mod raw;
pub mod utils;
