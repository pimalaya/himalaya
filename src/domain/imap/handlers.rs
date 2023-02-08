//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::{Context, Result};
use himalaya_lib::ImapBackend;

pub fn notify(imap: &ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.notify(keepalive, folder).context("cannot imap notify")
}

pub fn watch(imap: &ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.watch(keepalive, folder).context("cannot imap watch")
}
