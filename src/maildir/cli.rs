use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    client::MaildirClient, create::MaildirMailboxCreateCommand,
    delete::MaildirMailboxDeleteCommand, envelope::cli::MaildirEnvelopeCommand,
    flag::cli::MaildirFlagCommand, list::MaildirMailboxListCommand,
    message::cli::MaildirMessageCommand, rename::MaildirMailboxRenameCommand,
};

/// Maildir CLI.
///
/// This command gives you access to the Maildir CLI API, and allows
/// you to manage Maildir mailboxes, envelopes, flags, messages etc.
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
    #[command(subcommand)]
    Envelopes(MaildirEnvelopeCommand),
}

impl MaildirCommand {
    pub fn execute(self, printer: &mut impl Printer, client: MaildirClient) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, client),

            Self::Messages(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, client),
            Self::Envelopes(cmd) => cmd.execute(printer, client),
        }
    }
}
