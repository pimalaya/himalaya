use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    envelope::{
        get::ImapEnvelopeGetCommand, list::ImapEnvelopeListCommand,
        search::ImapEnvelopeSearchCommand, sort::ImapEnvelopeSortCommand,
        thread::ImapEnvelopeThreadCommand,
    },
};

/// Manage IMAP envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc. This subcommand allows you
/// to get, list, search, sort, and thread envelopes.
#[derive(Debug, Subcommand)]
pub enum ImapEnvelopeCommand {
    Get(ImapEnvelopeGetCommand),
    List(ImapEnvelopeListCommand),
    Search(ImapEnvelopeSearchCommand),
    Sort(ImapEnvelopeSortCommand),
    Thread(ImapEnvelopeThreadCommand),
}

impl ImapEnvelopeCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Search(cmd) => cmd.execute(printer, account),
            Self::Sort(cmd) => cmd.execute(printer, account),
            Self::Thread(cmd) => cmd.execute(printer, account),
        }
    }
}
