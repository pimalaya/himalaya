//! # Envelope command
//!
//! The `envelope` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        envelope::{list::EnvelopeListCommand, search::EnvelopeSearchCommand},
    },
};

/// Manage envelopes using the shared API.
///
/// An envelope is a small subset of a message's headers, enough to say
/// what the message is about without fetching it.
#[derive(Debug, Subcommand)]
pub enum EnvelopeCommand {
    #[command(visible_alias = "ls")]
    List(EnvelopeListCommand),
    #[command(visible_alias = "sr")]
    Search(EnvelopeSearchCommand),
}

impl EnvelopeCommand {
    /// Runs the subcommand against the active account's client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Search(cmd) => cmd.execute(printer, account, client),
        }
    }
}
