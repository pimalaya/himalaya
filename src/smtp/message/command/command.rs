use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{config::SmtpConfig, smtp::message::command::send::SendMessageCommand};

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
    pub fn execute(self, printer: &mut impl Printer, config: SmtpConfig) -> Result<()> {
        match self {
            Self::Send(cmd) => cmd.execute(printer, config),
        }
    }
}
