//! # Shared API
//!
//! The cross-protocol commands, behaving the same whatever backend serves
//! the active account, and the client dispatching them onto it.
//!
//! The command tree is the same in every build carrying a mail transport,
//! `any(backend, feature = "smtp")`, whichever backends it compiles: a
//! verb no compiled backend serves still parses, and the resolver of
//! [`crate::capability`] refuses it under `capability-missing`.

#[cfg(any(backend, feature = "smtp"))]
pub mod attachment;
#[cfg(any(backend, feature = "smtp"))]
pub mod base64;
#[cfg(any(backend, feature = "smtp"))]
pub mod client;
#[cfg(any(backend, feature = "smtp", feature = "sieve"))]
pub mod crlf;
#[cfg(any(backend, feature = "smtp"))]
pub mod envelope;
#[cfg(any(backend, feature = "smtp"))]
pub mod flag;
#[cfg(any(backend, feature = "smtp"))]
pub mod mailbox;
#[cfg(any(backend, feature = "smtp"))]
pub mod message;
#[cfg(any(feature = "gmail", feature = "msgraph"))]
pub mod output;
#[cfg(any(feature = "imap", feature = "smtp", feature = "sieve"))]
pub mod raw;
#[cfg(any(
    feature = "gmail",
    feature = "msgraph",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "m2dir",
    feature = "sieve"
))]
pub mod receipt;
#[cfg(any(backend, feature = "smtp"))]
pub mod vacation;
