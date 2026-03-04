pub mod copy;
pub mod delete;
pub mod export;
pub mod get;
pub mod r#move;
pub mod read;
pub mod save;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::message::command::{
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
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, config),
            Self::Get(cmd) => cmd.execute(printer, config),
            Self::Read(cmd) => cmd.execute(printer, config),
            Self::Export(cmd) => cmd.execute(printer, config),
            Self::Copy(cmd) => cmd.execute(printer, config),
            Self::Move(cmd) => cmd.execute(printer, config),
            Self::Delete(cmd) => cmd.execute(printer, config),
        }
    }
}
