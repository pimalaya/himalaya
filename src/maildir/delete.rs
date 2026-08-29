//! # Maildir delete
//!
//! The `maildir delete` command, removing a Maildir and its messages.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::RequiredMaildirPathFlag,
    client::{MaildirClient, validate_maildir_name},
};

/// Delete a Maildir folder.
///
/// The directory and every message in it go. The target is named
/// explicitly, with no default, deletion being destructive.
#[derive(Debug, Parser)]
pub struct MaildirMailboxDeleteCommand {
    #[command(flatten)]
    pub maildir_path: RequiredMaildirPathFlag,
}

impl MaildirMailboxDeleteCommand {
    /// Deletes the Maildir and every message in it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(&self.maildir_path.inner)?;

        // NOTE: io-maildir resolves a name relative to the store root, so
        // the bare name goes through: pre-joining the root would look for
        // the Maildir under a second copy of it.
        let path = self.maildir_path.inner.to_string_lossy().into_owned();
        client.delete_maildir(path)?;
        printer.out(Message::new("Maildir successfully deleted"))
    }
}
