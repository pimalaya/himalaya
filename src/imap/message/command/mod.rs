pub mod copy;
pub mod delete;
pub mod r#move;
pub mod save;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::message::command::{
        copy::CopyMessageCommand, delete::DeleteMessageCommand, r#move::MoveMessageCommand,
        save::SaveMessageCommand,
    },
};

/// Manage messages.
///
/// A message is a complete email including headers and body. This
/// subcommand allows you to save, copy, move, and delete messages.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    Save(SaveMessageCommand),
    Copy(CopyMessageCommand),
    Move(MoveMessageCommand),
    Delete(DeleteMessageCommand),
}

impl MessageCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Save(cmd) => cmd.execute(printer, config),
            Self::Copy(cmd) => cmd.execute(printer, config),
            Self::Move(cmd) => cmd.execute(printer, config),
            Self::Delete(cmd) => cmd.execute(printer, config),
        }
    }
}
