//! # Flag command
//!
//! The `flag` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        flag::{add::FlagAddCommand, remove::FlagRemoveCommand, set::FlagSetCommand},
    },
};

/// Manage flags using the shared API.
///
/// A flag acts like a tag, saying what state or kind a message is in.
#[derive(Debug, Subcommand)]
pub enum FlagCommand {
    Add(FlagAddCommand),
    Set(FlagSetCommand),
    #[command(visible_alias = "rm")]
    Remove(FlagRemoveCommand),
}

impl FlagCommand {
    /// Runs the subcommand against the active account's client.
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
