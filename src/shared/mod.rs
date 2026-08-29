//! # Shared API
//!
//! The cross-protocol commands, behaving the same whatever backend serves
//! the active account, and the client dispatching them onto it.
//!
//! The `backend` cfg build.rs sets is on when a mailbox backend is
//! compiled in. Most commands need one, but composing and sending also
//! work over SMTP alone, so those carry `any(backend, feature = "smtp")`.

#[cfg(backend)]
pub mod attachment;
#[cfg(any(backend, feature = "smtp"))]
pub mod client;
pub mod crlf;
#[cfg(backend)]
pub mod envelope;
#[cfg(backend)]
pub mod flag;
#[cfg(backend)]
pub mod mailbox;
pub mod message;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
pub mod raw;
pub mod table;
