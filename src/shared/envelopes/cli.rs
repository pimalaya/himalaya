use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    envelopes::{list::EnvelopeListCommand, search::EnvelopeSearchCommand},
};

/// Shared API to manage envelopes for the active account.
///
/// An envelope is a message headers subset. It is usually small, and
/// contains enough information to have an overall understanding of
/// what a message is about.
#[derive(Debug, Subcommand)]
pub enum EnvelopeCommand {
    #[command(visible_alias = "ls")]
    List(EnvelopeListCommand),
    #[command(visible_alias = "sr")]
    Search(EnvelopeSearchCommand),
}

impl EnvelopeCommand {
    pub fn execute(self, printer: &mut impl Printer, client: EmailClient) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
            Self::Search(cmd) => cmd.execute(printer, client),
        }
    }
}
