use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    client::MaildirClient,
    message::{
        copy::MaildirMessageCopyCommand, r#move::MaildirMessageMoveCommand,
        save::MaildirMessageSaveCommand,
    },
};

/// Manage MAILDIR messages.
///
/// A message is a file under the Maildir's `new` / `cur` subdirectories.
/// This subcommand stores and relocates those files; rendering their
/// content (headers, body, parts) is the job of the shared `messages`
/// and `envelopes` commands.
#[derive(Debug, Subcommand)]
pub enum MaildirMessageCommand {
    Save(MaildirMessageSaveCommand),
    Copy(MaildirMessageCopyCommand),
    Move(MaildirMessageMoveCommand),
}

impl MaildirMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MaildirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
        }
    }
}
