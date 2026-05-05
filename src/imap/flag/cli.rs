use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    flag::{
        add::ImapFlagAddCommand, list::ImapFlagListCommand, remove::ImapFlagRemoveCommand,
        set::ImapFlagSetCommand,
    },
};

/// Manage IMAP flags.
///
/// A flag is a label attached to a message. This subcommand allows
/// you to manage them.
#[derive(Debug, Subcommand)]
pub enum ImapFlagCommand {
    List(ImapFlagListCommand),
    Add(ImapFlagAddCommand),
    Set(ImapFlagSetCommand),
    Remove(ImapFlagRemoveCommand),
}

impl ImapFlagCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Add(cmd) => cmd.execute(printer, account),
            Self::Set(cmd) => cmd.execute(printer, account),
            Self::Remove(cmd) => cmd.execute(printer, account),
        }
    }
}
