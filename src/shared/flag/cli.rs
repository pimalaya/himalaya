use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    flag::{add::FlagAddCommand, remove::FlagRemoveCommand, set::FlagSetCommand},
};

/// Manage flags using the shared API.
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.execute(printer, account, client),
            Self::Set(cmd) => cmd.execute(printer, account, client),
            Self::Remove(cmd) => cmd.execute(printer, account, client),
        }
    }
}
