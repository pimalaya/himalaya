//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::{Context, Result};
use himalaya_lib::backend::ImapBackend;

pub fn notify(keepalive: u64, mbox: &str, imap: &mut ImapBackend) -> Result<()> {
    imap.notify(keepalive, mbox).context("cannot imap notify")
}

pub fn watch(keepalive: u64, mbox: &str, imap: &mut ImapBackend) -> Result<()> {
    imap.watch(keepalive, mbox).context("cannot imap watch")
}
