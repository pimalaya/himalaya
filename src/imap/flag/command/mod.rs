pub mod add;
pub mod list;
pub mod remove;
pub mod set;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::flag::command::{
        add::AddFlagsCommand, list::ListFlagsCommand, remove::RemoveFlagsCommand,
        set::SetFlagsCommand,
    },
};

/// Manage message flags.
///
/// A flag is a label attached to a message. This subcommand allows
/// you to manage them: list available flags, add flags to messages,
/// remove flags from messages, etc.
#[derive(Debug, Subcommand)]
pub enum FlagCommand {
    List(ListFlagsCommand),
    Add(AddFlagsCommand),
    Set(SetFlagsCommand),
    Remove(RemoveFlagsCommand),
}

impl FlagCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config),
            Self::Add(cmd) => cmd.execute(printer, config),
            Self::Set(cmd) => cmd.execute(printer, config),
            Self::Remove(cmd) => cmd.execute(printer, config),
        }
    }
}
