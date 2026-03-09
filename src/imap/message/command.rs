use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    message::{
        copy::CopyMessageCommand, export::ExportMessageCommand, get::GetMessageCommand,
        r#move::MoveMessageCommand, read::ReadMessageCommand, save::SaveMessageCommand,
    },
};

/// Manage IMAP messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, and move
/// messages.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    Save(SaveMessageCommand),
    Get(GetMessageCommand),
    Read(ReadMessageCommand),
    Export(ExportMessageCommand),
    Copy(CopyMessageCommand),
    Move(MoveMessageCommand),
}

impl MessageCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.exec(printer, account),
            Self::Get(cmd) => cmd.exec(printer, account),
            Self::Read(cmd) => cmd.exec(printer, account),
            Self::Export(cmd) => cmd.exec(printer, account),
            Self::Copy(cmd) => cmd.exec(printer, account),
            Self::Move(cmd) => cmd.exec(printer, account),
        }
    }
}
