//! # SMTP raw
//!
//! The `smtp raw` command, a byte-for-byte passthrough to the server.

use anyhow::{Result, bail};
use clap::Parser;
use io_smtp::client::SmtpClient as _;
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::raw::RawCommandArg, smtp::client::SmtpClient};

/// Send a raw SMTP command and print the verbatim server reply.
///
/// One line goes out without its trailing CRLF, which io-smtp appends
/// before reading the full reply back. Any reply code comes back as output
/// rather than as an error.
///
/// The exchange reads exactly one reply, so batching is refused, and
/// `DATA` and `STARTTLS`, which switch the stream into another mode, are
/// not supported.
#[derive(Debug, Parser)]
pub struct SmtpRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SmtpRawCommand {
    /// Sends the command and prints the raw reply.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        // NOTE: io-smtp appends the trailing CRLF itself, so the one the
        // caller may have added is stripped.
        let command = self.command.parse()?;
        let command = command.trim_end_matches(['\r', '\n']);

        // NOTE: an interior newline would be a second command the reply
        // parser never accounts for, desyncing the stream.
        if command.contains('\n') {
            bail!("SMTP raw accepts a single command line; batching is not supported");
        }

        let reply = client.raw(command.to_string().into())?;

        printer.out(Message::new(reply))
    }
}
