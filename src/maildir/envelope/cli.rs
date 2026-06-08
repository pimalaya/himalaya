use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::maildir::{
    client::MaildirClient,
    envelope::{get::MaildirEnvelopeGetCommand, list::MaildirEnvelopeListCommand},
};

/// Manage MAILDIR envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc. This subcommand allows you
/// to get, list, search, sort, and thread envelopes.
#[derive(Debug, Subcommand)]
pub enum MaildirEnvelopeCommand {
    Get(MaildirEnvelopeGetCommand),
    List(MaildirEnvelopeListCommand),
}

impl MaildirEnvelopeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
        }
    }
}
