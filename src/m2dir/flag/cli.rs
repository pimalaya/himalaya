use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::m2dir::{
    client::M2dirClient,
    flag::{
        add::M2dirFlagAddCommand, list::M2dirFlagListCommand, remove::M2dirFlagRemoveCommand,
        set::M2dirFlagSetCommand,
    },
};

/// Manage M2DIR flags.
///
/// A flag is a free-form UTF-8 string stored in the
/// `.meta/<id>.flags` metadata file alongside the message.
#[derive(Debug, Subcommand)]
pub enum M2dirFlagCommand {
    List(M2dirFlagListCommand),
    Add(M2dirFlagAddCommand),
    Set(M2dirFlagSetCommand),
    Remove(M2dirFlagRemoveCommand),
}

impl M2dirFlagCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
            Self::Remove(cmd) => cmd.execute(printer, client),
        }
    }
}
