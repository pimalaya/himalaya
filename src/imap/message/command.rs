use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::message::{
        copy::CopyMessageCommand, delete::DeleteMessageCommand, export::ExportMessageCommand,
        get::GetMessageCommand, r#move::MoveMessageCommand, read::ReadMessageCommand,
        save::SaveMessageCommand,
    },
};

/// Manage messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, get, read, export, copy, move, and
/// delete messages.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    Save(SaveMessageCommand),
    Get(GetMessageCommand),
    Read(ReadMessageCommand),
    Export(ExportMessageCommand),
    Copy(CopyMessageCommand),
    Move(MoveMessageCommand),
    Delete(DeleteMessageCommand),
}

impl MessageCommand {
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.exec(printer, config),
            Self::Get(cmd) => cmd.exec(printer, config),
            Self::Read(cmd) => cmd.exec(printer, config),
            Self::Export(cmd) => cmd.exec(printer, config),
            Self::Copy(cmd) => cmd.exec(printer, config),
            Self::Move(cmd) => cmd.exec(printer, config),
            Self::Delete(cmd) => cmd.exec(printer, config),
        }
    }
}
