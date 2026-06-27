use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::client::ImapClient;

/// Send a raw IMAP command and print the verbatim server response.
///
/// The command is sent as-is, without tag and without trailing CRLF
/// (e.g. `CAPABILITY` or `SEARCH FROM "foo@bar"`); a tag is prepended
/// and the response is read up to and including the matching tagged
/// completion line. Tagged NO/BAD replies are returned as output, not
/// errors. Synchronizing literals in the command are not supported.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    /// The raw command line, without tag and without trailing CRLF.
    #[arg(value_name = "COMMAND")]
    pub command: String,
}

impl ImapRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let response = client.raw(self.command)?;

        printer.out(Message::new(response))
    }
}
