use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    envelope::{list::EnvelopeListCommand, search::EnvelopeSearchCommand},
};

/// Manage envelopes using the shared API.
///
/// An envelope is a message headers subset. It is usually small, and contains
/// enough information to have an overall understanding of what a message is
/// about.
#[derive(Debug, Subcommand)]
pub enum EnvelopeCommand {
    #[command(visible_alias = "ls")]
    List(EnvelopeListCommand),
    #[command(visible_alias = "sr")]
    Search(EnvelopeSearchCommand),
}

impl EnvelopeCommand {
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
