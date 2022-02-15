//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{config::AccountConfig, domain::imap::BackendService};

pub fn notify<'a, B: BackendService<'a>>(
    keepalive: u64,
    config: &AccountConfig,
    backend: &mut B,
) -> Result<()> {
    backend.notify(config, keepalive)
}

pub fn watch<'a, B: BackendService<'a>>(
    keepalive: u64,
    account: &AccountConfig,
    backend: &mut B,
) -> Result<()> {
    backend.watch(account, keepalive)
}
