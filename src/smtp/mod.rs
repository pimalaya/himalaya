//! # SMTP
//!
//! The `smtp` command family, plus the send-only transport the unified
//! commands use when their storage backend cannot send.

pub mod backend;
pub mod cli;
pub mod client;
pub mod raw;
pub mod send;
