//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::backends::ImapBackend;

pub fn notify(keepalive: u64, mbox: &str, imap: &mut ImapBackend) -> Result<()> {
    imap.notify(keepalive, mbox)
}

pub fn watch(keepalive: u64, mbox: &str, imap: &mut ImapBackend) -> Result<()> {
    imap.watch(keepalive, mbox)
}
