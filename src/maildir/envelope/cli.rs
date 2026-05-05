use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    account::MaildirAccount,
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
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::List(cmd) => cmd.execute(printer, account),
        }
    }
}
