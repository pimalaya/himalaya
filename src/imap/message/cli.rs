use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::imap::{client::ImapClient, message::get::ImapMessageGetCommand};

/// Fetch IMAP message data (FETCH, RFC 3501).
///
/// Currently exposes `get`, which fetches a message's BODYSTRUCTURE and
/// envelope to show its MIME structure.
#[derive(Debug, Subcommand)]
pub enum ImapMessageCommand {
    Get(ImapMessageGetCommand),
}

impl ImapMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
        }
    }
}
