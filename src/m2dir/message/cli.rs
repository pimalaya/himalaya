//! # m2dir message command
//!
//! The `m2dir message` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::m2dir::{client::M2dirClient, message::save::M2dirMessageSaveCommand};

/// Manage m2dir messages.
///
/// A message is a file inside an m2dir folder, and this stores one.
/// Rendering its content belongs to the shared `message` and `envelope`
/// commands.
#[derive(Debug, Subcommand)]
pub enum M2dirMessageCommand {
    Save(M2dirMessageSaveCommand),
}

impl M2dirMessageCommand {
    /// Runs the subcommand against the account's m2dir store.
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
        }
    }
}
