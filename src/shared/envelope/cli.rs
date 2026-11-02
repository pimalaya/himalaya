//! # Envelope command
//!
//! The `envelope` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    capability::{self, Capability},
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
    #[command(visible_alias = "ls", after_long_help = capability::help("envelope", "list"))]
    List(EnvelopeListCommand),
    #[command(visible_alias = "sr", after_long_help = capability::help("envelope", "search"))]
    Search(EnvelopeSearchCommand),
}

impl EnvelopeCommand {
    /// The capabilities the subcommand reaches, checked before any
    /// connection.
    pub fn capabilities(&self) -> Result<Vec<Capability>> {
        match self {
            Self::List(_) => Ok(vec![Capability::EnvelopeList]),
            Self::Search(cmd) => cmd.capabilities(),
        }
    }

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
