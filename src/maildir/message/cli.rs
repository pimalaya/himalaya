use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    client::MaildirClient,
    message::{
        copy::MaildirMessageCopyCommand, export::MaildirMessageExportCommand,
        get::MaildirMessageGetCommand, r#move::MaildirMessageMoveCommand,
        read::MaildirMessageReadCommand, save::MaildirMessageSaveCommand,
    },
};

/// Manage MAILDIR messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, and move
/// messages.
#[derive(Debug, Subcommand)]
pub enum MaildirMessageCommand {
    Save(MaildirMessageSaveCommand),
    Get(MaildirMessageGetCommand),
    Read(MaildirMessageReadCommand),
    Export(MaildirMessageExportCommand),
    Copy(MaildirMessageCopyCommand),
    Move(MaildirMessageMoveCommand),
}

impl MaildirMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Read(cmd) => cmd.execute(printer, client),
            Self::Export(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),
        }
    }
}
