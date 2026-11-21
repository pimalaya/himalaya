//! # Unified API
//!
//! The cross-protocol commands, behaving the same whatever backend serves
//! the active account, their domain types and the client dispatching them.
//!
//! The command tree is the same in every build carrying a mail transport,
//! `any(backend, feature = "smtp")`, whichever backends it compiles: a
//! verb no compiled backend serves still parses, and the resolver of
//! [`capability`] refuses it under `capability-missing`.

#[cfg(any(backend, feature = "smtp"))]
pub mod attachment;
pub mod backend;
pub mod capability;
#[cfg(any(backend, feature = "smtp"))]
pub mod client;
#[cfg(any(backend, feature = "smtp"))]
pub mod email;
#[cfg(any(backend, feature = "smtp"))]
pub mod envelope;
#[cfg(any(backend, feature = "smtp"))]
pub mod flag;
#[cfg(any(backend, feature = "smtp"))]
pub mod mailbox;
#[cfg(any(backend, feature = "smtp"))]
pub mod message;
#[cfg(any(backend, feature = "smtp"))]
pub mod vacation;
