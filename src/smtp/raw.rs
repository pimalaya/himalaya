use anyhow::{Result, bail};
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{shared::raw::RawCommandArg, smtp::client::SmtpClient};

/// Send a raw SMTP command and print the verbatim server reply.
///
/// The command is a single line sent without trailing CRLF (e.g. `NOOP`,
/// `VRFY foo@bar`, `HELP`); io-smtp appends the CRLF and reads the full
/// reply back. Any reply code, including 4xx and 5xx, is returned as
/// output rather than an error. Reserved for simple request/reply
/// commands; `DATA` and `STARTTLS`, which switch the stream into a
/// different mode, are not supported, and batching several commands is
/// not possible (the exchange reads exactly one reply).
#[derive(Debug, Parser)]
pub struct SmtpRawCommand {
    #[command(flatten)]
    pub command: RawCommandArg,
}

impl SmtpRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        // NOTE: io-smtp appends the trailing CRLF itself, so strip the
        // one the caller may have added (literal or real).
        let command = self.command.parse()?;
        let command = command.trim_end_matches(['\r', '\n']);

        // NOTE: the exchange sends one command line and reads one reply,
        // so an interior newline would be a second command the reply
        // parser never accounts for, desyncing the stream.
        if command.contains('\n') {
            bail!("SMTP raw accepts a single command line; batching is not supported");
        }

        let reply = client.raw(command.to_string())?;

        printer.out(Message::new(reply))
    }
}
