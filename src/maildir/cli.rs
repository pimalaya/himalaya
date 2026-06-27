use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::maildir::{
    client::MaildirClient, create::MaildirMailboxCreateCommand,
    delete::MaildirMailboxDeleteCommand, flag::cli::MaildirFlagCommand,
    list::MaildirMailboxListCommand, message::cli::MaildirMessageCommand,
    rename::MaildirMailboxRenameCommand,
};

/// Maildir-specific API.
///
/// This command gives you access to the raw Maildir API.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MaildirCommand {
    Create(MaildirMailboxCreateCommand),
    Rename(MaildirMailboxRenameCommand),
    Delete(MaildirMailboxDeleteCommand),
    List(MaildirMailboxListCommand),

    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(MaildirMessageCommand),
    #[command(subcommand)]
    Flags(MaildirFlagCommand),
}

impl MaildirCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, account, client),

            Self::Messages(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, account, client),
        }
    }
}
