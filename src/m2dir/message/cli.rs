use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::m2dir::{
    client::M2dirClient,
    message::{
        export::M2dirMessageExportCommand, get::M2dirMessageGetCommand,
        read::M2dirMessageReadCommand, save::M2dirMessageSaveCommand,
    },
};

/// Manage M2DIR messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read and export messages.
#[derive(Debug, Subcommand)]
pub enum M2dirMessageCommand {
    Save(M2dirMessageSaveCommand),
    Get(M2dirMessageGetCommand),
    Read(M2dirMessageReadCommand),
    Export(M2dirMessageExportCommand),
}

impl M2dirMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Read(cmd) => cmd.execute(printer, client),
            Self::Export(cmd) => cmd.execute(printer, client),
        }
    }
}
