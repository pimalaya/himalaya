use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::flag::{
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
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.exec(printer, config),
            Self::Add(cmd) => cmd.exec(printer, config),
            Self::Set(cmd) => cmd.exec(printer, config),
            Self::Remove(cmd) => cmd.exec(printer, config),
        }
    }
}
