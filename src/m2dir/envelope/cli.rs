use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::m2dir::{
    client::M2dirClient,
    envelope::{get::M2dirEnvelopeGetCommand, list::M2dirEnvelopeListCommand},
};

/// Manage M2DIR envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc.
#[derive(Debug, Subcommand)]
pub enum M2dirEnvelopeCommand {
    Get(M2dirEnvelopeGetCommand),
    List(M2dirEnvelopeListCommand),
}

impl M2dirEnvelopeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
        }
    }
}
