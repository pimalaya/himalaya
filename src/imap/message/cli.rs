use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    message::{
        copy::ImapMessageCopyCommand, export::ImapMessageExportCommand, get::ImapMessageGetCommand,
        r#move::ImapMessageMoveCommand, read::ImapMessageReadCommand, save::ImapMessageSaveCommand,
    },
};

/// Manage IMAP messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, and move
/// messages.
#[derive(Debug, Subcommand)]
pub enum ImapMessageCommand {
    Save(ImapMessageSaveCommand),
    Get(ImapMessageGetCommand),
    Read(ImapMessageReadCommand),
    Export(ImapMessageExportCommand),
    Copy(ImapMessageCopyCommand),
    Move(ImapMessageMoveCommand),
}

impl ImapMessageCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, account),
            Self::Get(cmd) => cmd.execute(printer, account),
            Self::Read(cmd) => cmd.execute(printer, account),
            Self::Export(cmd) => cmd.execute(printer, account),
            Self::Copy(cmd) => cmd.execute(printer, account),
            Self::Move(cmd) => cmd.execute(printer, account),
        }
    }
}
