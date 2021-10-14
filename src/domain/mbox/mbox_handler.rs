//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use log::trace;

use crate::{domain::ImapServiceInterface, output::OutputServiceInterface};

/// List all mailboxes.
pub fn list<'a, OutputService: OutputServiceInterface, ImapService: ImapServiceInterface<'a>>(
    output: &OutputService,
    imap: &'a mut ImapService,
) -> Result<()> {
    let mboxes = imap.fetch_mboxes()?;
    trace!("mailboxes: {:#?}", mboxes);
    output.print(mboxes)
}
