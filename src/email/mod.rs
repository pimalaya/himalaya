//! # Email
//!
//! The cross-protocol email domain types, the least-common-denominator
//! shapes the shared subcommands render.
//!
//! They were inlined from the retired io-email crate, the CLI owning its
//! shared types and a per-backend dispatching client instead. The adapters
//! producing them live in each protocol module's backend submodule.

pub mod address;
pub mod envelope;
pub mod flag;
pub mod mailbox;
pub mod search;
