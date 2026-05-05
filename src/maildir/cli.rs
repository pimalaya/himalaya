use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::maildir::{
    account::MaildirAccount, create::MaildirMailboxCreateCommand,
    delete::MaildirMailboxDeleteCommand, envelope::cli::MaildirEnvelopeCommand,
    flag::cli::MaildirFlagCommand, list::MaildirMailboxListCommand,
    message::cli::MaildirMessageCommand, rename::MaildirMailboxRenameCommand,
};

/// MAILDIR CLI (requires the `maildir` cargo feature).
///
/// This command gives you access to the MAILDIR CLI API, and allows you
/// to manage MAILDIR mailboxes, envelopes, flags, messages etc.
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
    pub fn execute(self, printer: &mut impl Printer, account: MaildirAccount) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Rename(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
            Self::List(cmd) => cmd.execute(printer, account),

            Self::Messages(cmd) => cmd.execute(printer, account),
            Self::Flags(cmd) => cmd.execute(printer, account),
            Self::Envelopes(cmd) => cmd.execute(printer, account),
        }
    }
}
