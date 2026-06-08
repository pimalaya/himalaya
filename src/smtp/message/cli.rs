use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::smtp::{client::SmtpClient, message::send::SmtpMessageSendCommand};

/// Manage messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, move, and
/// delete messages.
#[derive(Debug, Subcommand)]
pub enum SmtpMessageCommand {
    Send(SmtpMessageSendCommand),
}

impl SmtpMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut SmtpClient) -> Result<()> {
        match self {
            Self::Send(cmd) => cmd.execute(printer, client),
        }
    }
}
