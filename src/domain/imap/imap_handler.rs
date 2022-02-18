//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{config::AccountConfig, domain::imap::ImapService};

pub fn notify(
    keepalive: u64,
    mbox: &str,
    config: &AccountConfig,
    imap: &mut ImapService,
) -> Result<()> {
    imap.notify(keepalive, mbox, config)
}

pub fn watch(
    keepalive: u64,
    mbox: &str,
    config: &AccountConfig,
    imap: &mut ImapService,
) -> Result<()> {
    imap.watch(keepalive, mbox, config)
}
