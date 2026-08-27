use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    maildir::{
        client::MaildirClient, create::MaildirMailboxCreateCommand,
        delete::MaildirMailboxDeleteCommand, flag::cli::MaildirFlagCommand,
        list::MaildirMailboxListCommand, message::cli::MaildirMessageCommand,
        rename::MaildirMailboxRenameCommand,
    },
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
    #[command(visible_alias = "msg", aliases = ["messages", "msgs"])]
    Message(MaildirMessageCommand),
    #[command(subcommand)]
    #[command(alias = "flags")]
    Flag(MaildirFlagCommand),
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

            Self::Message(cmd) => cmd.execute(printer, client),
            Self::Flag(cmd) => cmd.execute(printer, account, client),
        }
    }
}
