//! # Gmail
//!
//! The `gmail` command family, one subcommand group per Gmail REST
//! resource, plus the adapter serving the shared commands over Gmail.

pub mod attachments;
pub mod backend;
pub mod cli;
pub mod client;
pub mod drafts;
pub mod format;
pub mod history;
pub mod labels;
pub mod messages;
pub mod profile;
pub mod settings;
pub mod threads;
