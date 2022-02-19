//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{backends::ImapBackend, config::AccountConfig};

pub fn notify(
    keepalive: u64,
    mbox: &str,
    config: &AccountConfig,
    imap: &mut ImapBackend,
) -> Result<()> {
    imap.notify(keepalive, mbox, config)
}

pub fn watch(
    keepalive: u64,
    mbox: &str,
    config: &AccountConfig,
    imap: &mut ImapBackend,
) -> Result<()> {
    imap.watch(keepalive, mbox, config)
}
