use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::smtp::client::SmtpClient;

/// Send a raw SMTP command and print the verbatim server reply.
///
/// The command is sent as-is, without trailing CRLF (e.g. `NOOP`,
/// `VRFY foo@bar`, `HELP`), and the full reply is read back. Any reply
/// code, including 4xx and 5xx, is returned as output rather than an
/// error. Reserved for simple request/reply commands; `DATA` and
/// `STARTTLS`, which switch the stream into a different mode, are not
/// supported.
#[derive(Debug, Parser)]
pub struct SmtpRawCommand {
    /// The raw command line, without trailing CRLF.
    #[arg(value_name = "COMMAND")]
    pub command: String,
}

impl SmtpRawCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        let reply = client.raw(self.command)?;

        printer.out(Message::new(reply))
    }
}
