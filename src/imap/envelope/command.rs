use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    envelope::{
        get::GetEnvelopeCommand, list::ListEnvelopesCommand, search::SearchEnvelopesCommand,
        sort::SortEnvelopesCommand, thread::ThreadEnvelopesCommand,
    },
};

/// Manage message envelopes.
///
/// An envelope contains header information about a message such as
/// date, subject, from, to, cc, bcc, etc. This subcommand allows you
/// to list, get, search, sort, and thread envelopes.
#[derive(Debug, Subcommand)]
pub enum EnvelopeCommand {
    List(ListEnvelopesCommand),
    Get(GetEnvelopeCommand),
    Search(SearchEnvelopesCommand),
    Sort(SortEnvelopesCommand),
    Thread(ThreadEnvelopesCommand),
}

impl EnvelopeCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.exec(printer, account),
            Self::Get(cmd) => cmd.exec(printer, account),
            Self::Search(cmd) => cmd.exec(printer, account),
            Self::Sort(cmd) => cmd.exec(printer, account),
            Self::Thread(cmd) => cmd.exec(printer, account),
        }
    }
}
