use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    flags::{add::FlagAddCommand, remove::FlagRemoveCommand, set::FlagSetCommand},
};

/// Shared API to manage flags for the active account.
///
/// A flag is acting like a tag, giving information about message state or kind.
#[derive(Debug, Subcommand)]
pub enum FlagCommand {
    Add(FlagAddCommand),
    Set(FlagSetCommand),
    #[command(visible_alias = "rm")]
    Remove(FlagRemoveCommand),
}

impl FlagCommand {
    pub fn execute(self, printer: &mut impl Printer, client: EmailClient) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
            Self::Remove(cmd) => cmd.execute(printer, client),
        }
    }
}
