//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{
    config::{Account, Config},
    domain::imap::BackendService,
};

pub fn notify<'a, B: BackendService<'a>>(
    keepalive: u64,
    config: &Config,
    account: &Account,
    backend: &mut B,
) -> Result<()> {
    backend.notify(config, account, keepalive)
}

pub fn watch<'a, B: BackendService<'a>>(
    keepalive: u64,
    account: &Account,
    backend: &mut B,
) -> Result<()> {
    backend.watch(account, keepalive)
}
