//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use log::trace;

use crate::{
    domain::{ImapServiceInterface, Mboxes},
    output::OutputServiceInterface,
};

/// List all mailboxes.
pub fn list<OutputService: OutputServiceInterface, ImapService: ImapServiceInterface>(
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let raw_mboxes = imap.fetch_raw_mboxes()?;
    let mboxes = Mboxes::from(&raw_mboxes);
    trace!("mailboxes: {:#?}", mboxes);
    output.print(mboxes)
}
