//! # mbox flag command
//!
//! The `mbox flag` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    mbox::{
        client::MboxClient,
        flag::{
            add::MboxFlagAddCommand, list::MboxFlagListCommand, remove::MboxFlagRemoveCommand,
            set::MboxFlagSetCommand,
        },
    },
};

/// Manage mbox flags.
///
/// A flag is a letter stored in the `Status` or `X-Status` header of the
/// message. Changing one rewrites the file from that message on, under
/// lock.
#[derive(Debug, Subcommand)]
pub enum MboxFlagCommand {
    List(MboxFlagListCommand),
    Add(MboxFlagAddCommand),
    Set(MboxFlagSetCommand),
    Remove(MboxFlagRemoveCommand),
}

impl MboxFlagCommand {
    /// Runs the subcommand against the account's mbox files.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MboxClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
            Self::Remove(cmd) => cmd.execute(printer, client),
        }
    }
}
