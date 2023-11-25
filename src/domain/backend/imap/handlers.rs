//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

pub async fn notify(imap: &mut ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.notify(keepalive, folder).await?;
    Ok(())
}

pub async fn watch(imap: &mut ImapBackend, folder: &str, keepalive: u64) -> Result<()> {
    imap.watch(keepalive, folder).await?;
    Ok(())
}
