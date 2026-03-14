use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    flag::{
        add::AddFlagsCommand, list::ListFlagsCommand, remove::RemoveFlagsCommand,
        set::SetFlagsCommand,
    },
};

/// Manage IMAP flags.
///
/// A flag is a label attached to a message. This subcommand allows
/// you to manage them.
#[derive(Debug, Subcommand)]
pub enum FlagCommand {
    List(ListFlagsCommand),
    Add(AddFlagsCommand),
    Set(SetFlagsCommand),
    Remove(RemoveFlagsCommand),
}

impl FlagCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Add(cmd) => cmd.execute(printer, account),
            Self::Set(cmd) => cmd.execute(printer, account),
            Self::Remove(cmd) => cmd.execute(printer, account),
        }
    }
}
