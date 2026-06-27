use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::m2dir::{client::M2dirClient, message::save::M2dirMessageSaveCommand};

/// Manage M2DIR messages.
///
/// A message is a file stored inside an m2dir folder. This subcommand
/// stores them; rendering their content (headers, body, parts) is the
/// job of the shared `messages` and `envelopes` commands.
#[derive(Debug, Subcommand)]
pub enum M2dirMessageCommand {
    Save(M2dirMessageSaveCommand),
}

impl M2dirMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
        }
    }
}
