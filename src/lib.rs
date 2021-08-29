/// `comp` stands for `completion`. This module makes it possible to create autocompletion-settings
/// for himalaya for your shell :)
pub mod comp;

/// Everything which is related to the config files. For example the structure of your config file.
pub mod config;

/// A often used-struct to help us to access the most often used structs.
pub mod ctx;

/// A wrapper for representing a flag of a message or mailbox. For example the delete-flag or
/// read-flag.
pub mod flag;

/// A wrapper for creating connections easier to the IMAP-Servers.
pub mod imap;

/// Handles the input-interaction with the user. For example if you want to edit the body of your
/// message, his module takes care of the draft and calls your ~(neo)vim~ your favourite editor.
pub mod input;

/// Everything which is related to mboexes, for example creating or deleting some.
pub mod mbox;

/// Includes everything related to a message. This means: Body, Headers, Attachments, etc.
pub mod msg;

/// Handles the output. For example the JSON and HTML output.
pub mod output;

/// This module takes care for sending your mails!
pub mod smtp;

/// The TUI for listing the mails for example.
pub mod table;
