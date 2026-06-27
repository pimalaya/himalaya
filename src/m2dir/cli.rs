use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::m2dir::{
    client::M2dirClient, create::M2dirMailboxCreateCommand, delete::M2dirMailboxDeleteCommand,
    flag::cli::M2dirFlagCommand, list::M2dirMailboxListCommand, message::cli::M2dirMessageCommand,
};

/// M2dir-specific API.
///
/// This command gives you access to the raw m2dir API.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum M2dirCommand {
    Create(M2dirMailboxCreateCommand),
    Delete(M2dirMailboxDeleteCommand),
    List(M2dirMailboxListCommand),

    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(M2dirMessageCommand),
    #[command(subcommand)]
    Flags(M2dirFlagCommand),
}

impl M2dirCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, account, client),

            Self::Messages(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, account, client),
        }
    }
}
