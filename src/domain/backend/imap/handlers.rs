//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;
use pimalaya_email::ImapBackend;

pub fn notify(imap: &mut ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.notify(keepalive, folder)?;
    Ok(())
}

pub fn watch(imap: &mut ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.watch(keepalive, folder)?;
    Ok(())
}
