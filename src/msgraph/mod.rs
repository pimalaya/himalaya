//! # Microsoft Graph
//!
//! The `msgraph` command family, one subcommand group per Graph mail
//! resource, plus the adapter serving the shared commands over Graph.

pub mod attachments;
pub mod backend;
pub mod cli;
pub mod client;
pub mod mail_folders;
pub mod messages;
pub mod profile;
