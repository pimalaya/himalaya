use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::maildir::{
    client::MaildirClient,
    flag::{
        add::MaildirFlagAddCommand, list::MaildirFlagListCommand, remove::MaildirFlagRemoveCommand,
        set::MaildirFlagSetCommand,
    },
};

/// Manage MAILDIR flags.
///
/// A flag is a label attached to a message. This subcommand allows
/// you to manage them.
#[derive(Debug, Subcommand)]
pub enum MaildirFlagCommand {
    List(MaildirFlagListCommand),
    Add(MaildirFlagAddCommand),
    Set(MaildirFlagSetCommand),
    Remove(MaildirFlagRemoveCommand),
}

impl MaildirFlagCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Add(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
            Self::Remove(cmd) => cmd.execute(printer, client),
        }
    }
}
