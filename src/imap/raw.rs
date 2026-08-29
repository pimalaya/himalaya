//! # IMAP raw
//!
//! The `imap raw` command, a byte-for-byte passthrough to the server.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::{imap::client::ImapClient, shared::raw::RawCommandArg};

/// Send raw IMAP commands and print the verbatim server response.
///
/// The input goes out byte for byte, no tag added and no CRLF trimmed, so
/// every command carries its own tag and a CRLF separates them. That is
/// what lets a whole batch be pipelined at once.
///
/// A literal `\r` or `\n` typed on the shell becomes a real CRLF, and a
/// trailing one is appended when missing. The response is read until
/// every tagged completion has arrived, possibly out of order, and a
/// tagged NO or BAD comes back as output rather than as an error.
#[derive(Debug, Parser)]
pub struct ImapRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl ImapRawCommand {
    /// Sends the commands and prints the raw response.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mut command = self.command.parse()?;

        // NOTE: io-imap rejects an unterminated command, so the newline
        // the caller may have left off the last one is appended.
        if !command.ends_with('\n') {
            command.push('\n');
        }

        let response = client.raw(command.as_bytes())?;

        printer.out(Message::new(response))
    }
}
