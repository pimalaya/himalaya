//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{config::AccountConfig, domain::imap::ImapService};

pub fn notify(keepalive: u64, config: &AccountConfig, imap: &mut ImapService) -> Result<()> {
    imap.notify(config, keepalive)
}

pub fn watch(keepalive: u64, account: &AccountConfig, imap: &mut ImapService) -> Result<()> {
    imap.watch(account, keepalive)
}
