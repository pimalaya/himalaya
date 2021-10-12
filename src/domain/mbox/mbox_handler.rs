//! Mailbox handling module.
//!
//! This module gathers all mailbox actions triggered by the CLI.

use anyhow::Result;
use log::{debug, trace};

use crate::{
    domain::{ImapServiceInterface, Mboxes},
    output::{OutputService, OutputServiceInterface},
};

/// List all mailboxes.
pub fn list<ImapService: ImapServiceInterface>(
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let names = imap.get_mboxes()?;
    let mboxes = Mboxes::from(&names);
    debug!("mailboxes len: {}", mboxes.len());
    trace!("mailboxes: {:#?}", mboxes);
    output.print(mboxes)?;
    Ok(())
}
