//! Module related to mailboxes handling.
//!
//! This module gathers all mailboxes actions triggered by the CLI.

use anyhow::Result;
use log::{debug, trace};

use crate::{
    domain::{imap::service::ImapServiceInterface, mbox::entity::Mboxes},
    output::service::{OutputService, OutputServiceInterface},
};

/// List all mailboxes.
pub fn list<ImapService: ImapServiceInterface>(
    output: &OutputService,
    imap: &mut ImapService,
) -> Result<()> {
    let names = imap.get_mboxes()?;
    let mboxes = Mboxes::from(&names);
    debug!("mailboxes len: {}", mboxes.0.len());
    trace!("mailboxes: {:#?}", mboxes);
    output.print(mboxes)?;
    Ok(())
}
