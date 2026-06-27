use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
    envelope::{get::ImapEnvelopeGetCommand, list::ImapEnvelopeListCommand},
};

/// Fetch IMAP envelopes (FETCH ENVELOPE, RFC 3501).
///
/// An envelope is the parsed header summary (date, subject, from, to,
/// cc, bcc, ...) the server returns for the FETCH ENVELOPE item.
#[derive(Debug, Subcommand)]
pub enum ImapEnvelopeCommand {
    Get(ImapEnvelopeGetCommand),
    List(ImapEnvelopeListCommand),
}

impl ImapEnvelopeCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
        }
    }
}
