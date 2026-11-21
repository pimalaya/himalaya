//! # Message
//!
//! The `message` command family: reading a message, writing one, and
//! moving one between mailboxes.
//!
//! Compiled with any mail transport, SMTP alone included, a verb needing a
//! mailbox being refused by the resolver there. The protocol-specific send
//! and save commands take the `arg` input resolver too.

pub mod add;
pub mod arg;
pub mod builder;
pub mod cli;
pub mod compose;
pub mod copy;
pub mod delete;
pub mod forward;
pub mod handler;
pub mod header;
pub mod mv;
pub mod parse;
pub mod part;
pub mod read;
pub mod reply;
pub mod send;
