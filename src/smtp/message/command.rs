use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::smtp::{account::SmtpAccount, message::send::SendMessageCommand};

/// Manage messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, move, and
/// delete messages.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    Send(SendMessageCommand),
}

impl MessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: SmtpAccount) -> Result<()> {
        match self {
            Self::Send(cmd) => cmd.exec(printer, account),
        }
    }
}
