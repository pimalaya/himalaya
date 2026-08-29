//! # Message
//!
//! The `message` command family: reading a message, writing one, and
//! moving one between mailboxes.
//!
//! The `arg` input resolver is compiled unconditionally, the
//! protocol-specific send and save commands taking it too. Composing and
//! sending work over SMTP alone, where the rest needs a mailbox backend.

#[cfg(backend)]
pub mod add;
pub mod arg;
#[cfg(any(backend, feature = "smtp"))]
pub mod builder;
#[cfg(any(backend, feature = "smtp"))]
pub mod cli;
#[cfg(any(backend, feature = "smtp"))]
pub mod compose;
#[cfg(backend)]
pub mod copy;
#[cfg(backend)]
pub mod delete;
#[cfg(backend)]
pub mod forward;
#[cfg(any(backend, feature = "smtp"))]
pub mod handler;
#[cfg(backend)]
pub mod mv;
#[cfg(backend)]
pub mod read;
#[cfg(backend)]
pub mod reply;
#[cfg(any(backend, feature = "smtp"))]
pub mod send;
