//! # mbox message command
//!
//! The `mbox message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::mbox::{
    client::MboxClient,
    message::{
        copy::MboxMessageCopyCommand, r#move::MboxMessageMoveCommand, save::MboxMessageSaveCommand,
    },
};

/// Manage mbox messages.
///
/// A message is a `From_` line and the bytes up to the next one, and
/// these store and relocate one. Rendering its content belongs to the
/// shared `message` and `envelope` commands.
#[derive(Debug, Subcommand)]
pub enum MboxMessageCommand {
    Save(MboxMessageSaveCommand),
    Copy(MboxMessageCopyCommand),
    Move(MboxMessageMoveCommand),
}

impl MboxMessageCommand {
    /// Runs the subcommand against the account's mbox files.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MboxClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
        }
    }
}
