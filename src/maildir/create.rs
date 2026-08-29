//! # Maildir create
//!
//! The `maildir create` command, laying out a new Maildir under the
//! store root.

use std::path::Path;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::maildir::{
    arg::MaildirNameArg,
    client::{MaildirClient, validate_maildir_name},
};

/// Create a Maildir folder.
///
/// Its `cur`, `new` and `tmp` subdirectories are laid out under the
/// store root.
#[derive(Debug, Parser)]
pub struct MaildirMailboxCreateCommand {
    #[command(flatten)]
    pub maildir_name: MaildirNameArg,
}

impl MaildirMailboxCreateCommand {
    /// Creates the Maildir under the store root.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        validate_maildir_name(Path::new(&self.maildir_name.inner))?;

        // NOTE: io-maildir resolves a name relative to the store root, so
        // the bare name goes through: pre-joining the root would land the
        // Maildir under a second copy of it.
        client.create_maildir(self.maildir_name.inner)?;
        printer.out(Message::new("Maildir successfully created"))
    }
}
