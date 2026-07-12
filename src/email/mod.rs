//! Shared cross-protocol email domain types, inlined from the retiring
//! io-email crate (see docs/io-email-inlining.md).
//!
//! io-email is being removed as a dependency: the CLI owns its shared
//! types and a per-backend dispatching client instead, mirroring
//! cardamum's structure. These modules hold the least-common-denominator
//! shapes the shared subcommands render; the per-backend adapters that
//! produce them live in each protocol module's `backend` submodule.

pub mod address;
pub mod envelope;
pub mod flag;
pub mod mailbox;
pub mod search;
