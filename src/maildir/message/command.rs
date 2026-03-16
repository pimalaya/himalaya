use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::maildir::{
    account::MaildirAccount,
    message::{
        copy::CopyMessagesCommand, export::ExportMessageCommand, get::GetMessageCommand,
        r#move::MoveMessagesCommand, read::ReadMessageCommand, save::SaveMessageCommand,
    },
};

/// Manage MAILDIR messages.
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
    Copy(CopyMessagesCommand),
    Move(MoveMessagesCommand),
}

impl MessageCommand {
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
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
