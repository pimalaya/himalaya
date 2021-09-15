//! # Welcome to Himalaya!
//! Here's a little summary of how to read the code of himalaya:
//! Each module includes three "main" files:
//! - `model.rs`: **The "main" file** of each module which includes the main implementation of the given
//!     module.
//! - `cli.rs`: Includes the subcommands and arguments which are related to the module.
//!     
//!     For example the `read` subcommand is in the `msg/cli.rs` file because it's related to the
//!     msg you want to read.
//!
//! - `mod.rs`: Includes all other files in the module. Click [here] for more information.
//!
//! [here]: https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html

/// `comp` stands for `completion`. This module makes it possible to create autocompletion-settings
/// for himalaya for your shell :)
pub mod comp;

/// Everything which is related to the config files. For example the structure of your config file.
pub mod config;

/// A wrapper for representing a flag of a message or mailbox. For example the delete-flag or
/// read-flag.
pub mod flag;

/// A wrapper for creating connections easier to the IMAP-Servers.
pub mod imap;

/// Handles the input-interaction with the user. For example if you want to edit the body of your
/// message, his module takes care of the draft and calls your ~(neo)vim~ your favourite editor.
pub mod input;

/// Everything which is related to mboxes, for example creating or deleting some.
pub mod mbox;

/// Includes everything related to a message. This means: Body, Headers, Attachments, etc.
pub mod msg;

/// Handles the output. For example the JSON and HTML output.
pub mod output;

pub mod domain;
pub mod ui;
