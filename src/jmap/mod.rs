//! # JMAP
//!
//! The `jmap` command family, one subcommand group per RFC 8621 data
//! type, plus the adapter serving the shared commands over JMAP.

pub mod backend;
pub mod cli;
pub mod client;
pub mod email;
pub mod error;
pub mod identity;
pub mod mailbox;
pub mod query;
pub mod submission;
pub mod thread;
pub mod vacation;
