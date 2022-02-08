//! Module related to IMAP handling.
//!
//! This module gathers all IMAP handlers triggered by the CLI.

use anyhow::Result;

use crate::{
    config::{Account, Config},
    domain::imap::ImapServiceInterface,
};

pub fn notify<'a, ImapService: ImapServiceInterface<'a>>(
    keepalive: u64,
    config: &Config,
    account: &Account,
    imap: &mut ImapService,
) -> Result<()> {
    imap.notify(config, account, keepalive)
}

pub fn watch<'a, ImapService: ImapServiceInterface<'a>>(
    keepalive: u64,
    account: &Account,
    imap: &mut ImapService,
) -> Result<()> {
    imap.watch(account, keepalive)
}
