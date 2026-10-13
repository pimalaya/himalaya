//! # pimdir attachment command
//!
//! The `pimdir attachment` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    pimdir::{
        attachment::{list::PimdirAttachmentListCommand, search::PimdirAttachmentSearchCommand},
        client::PimdirClient,
    },
};

/// List and search the files the account's messages attach.
///
/// The shared `attachment list` reads the parts of one message. These
/// read every file of the account at once, from the store's index, each
/// with the message it came with, so a mailbox need not be walked.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum PimdirAttachmentCommand {
    #[command(alias = "ls")]
    List(PimdirAttachmentListCommand),
    Search(PimdirAttachmentSearchCommand),
}

impl PimdirAttachmentCommand {
    /// Runs the subcommand against the account's pimdir store.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Search(cmd) => cmd.execute(printer, account, client),
        }
    }
}
