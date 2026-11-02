//! # Message
//!
//! The `message` command family: reading a message, writing one, and
//! moving one between mailboxes.
//!
//! The `arg` input resolver is compiled unconditionally, the
//! protocol-specific send and save commands taking it too. The rest is
//! compiled with any mail transport, SMTP alone included, a verb needing
//! a mailbox being refused by the resolver there.

#[cfg(any(backend, feature = "smtp"))]
pub mod add;
pub mod arg;
#[cfg(any(backend, feature = "smtp"))]
pub mod builder;
#[cfg(any(backend, feature = "smtp"))]
pub mod cli;
#[cfg(any(backend, feature = "smtp"))]
pub mod compose;
#[cfg(any(backend, feature = "smtp"))]
pub mod copy;
#[cfg(any(backend, feature = "smtp"))]
pub mod delete;
#[cfg(any(backend, feature = "smtp"))]
pub mod forward;
#[cfg(any(backend, feature = "smtp"))]
pub mod handler;
#[cfg(any(backend, feature = "smtp"))]
pub mod header;
#[cfg(any(backend, feature = "smtp"))]
pub mod mv;
#[cfg(any(backend, feature = "smtp"))]
pub mod parse;
#[cfg(any(backend, feature = "smtp"))]
pub mod part;
#[cfg(any(backend, feature = "smtp"))]
pub mod read;
#[cfg(any(backend, feature = "smtp"))]
pub mod reply;
#[cfg(any(backend, feature = "smtp"))]
pub mod send;
