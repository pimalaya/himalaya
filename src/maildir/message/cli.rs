//! # Maildir message command
//!
//! The `maildir message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    client::MaildirClient,
    message::{
        add::MaildirMessageAddCommand, copy::MaildirMessageCopyCommand,
        r#move::MaildirMessageMoveCommand,
    },
};

/// Manage Maildir messages.
///
/// A message is a file under the Maildir's `new` or `cur` subdirectory,
/// and these store and relocate one. Rendering its content belongs to the
/// unified `message` and `envelope` commands.
#[derive(Debug, Subcommand)]
pub enum MaildirMessageCommand {
    #[command(alias = "save")]
    Add(MaildirMessageAddCommand),
    Copy(MaildirMessageCopyCommand),
    Move(MaildirMessageMoveCommand),
}

impl MaildirMessageCommand {
    /// Runs the subcommand against the account's Maildir store.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
        }
    }
}
