//! # pimdir mailbox command
//!
//! The `pimdir mailbox` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        mailbox::{
            create::PimdirMailboxCreateCommand, delete::PimdirMailboxDeleteCommand,
            list::PimdirMailboxListCommand, rename::PimdirMailboxRenameCommand,
        },
    },
};

/// List the store's mailboxes with their coverage, or ask the sync engine
/// to create, rename or delete one.
///
/// A pimdir store mirrors its server, so a mailbox is created, renamed or
/// deleted there by the sync engine, and the change arrives in the store
/// with the sync that performs it.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PimdirMailboxCommand {
    List(PimdirMailboxListCommand),
    Create(PimdirMailboxCreateCommand),
    Rename(PimdirMailboxRenameCommand),
    Delete(PimdirMailboxDeleteCommand),
}

impl PimdirMailboxCommand {
    /// Runs the subcommand against the account's pimdir store.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, account, client),
            Self::Rename(cmd) => cmd.execute(printer, account, client),
            Self::Delete(cmd) => cmd.execute(printer, account, client),
        }
    }
}
