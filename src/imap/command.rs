use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount, envelope::command::ImapEnvelopeCommand, flag::command::ImapFlagCommand,
    id::ImapIdCommand, mailbox::command::ImapMailboxCommand, message::command::ImapMessageCommand,
};

/// IMAP CLI (requires the `imap` cargo feature).
///
/// This command gives you access to the IMAP CLI API, and allows you
/// to manage IMAP mailboxes, envelopes, flags, messages etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ImapCommand {
    Id(ImapIdCommand),

    #[command(subcommand)]
    #[command(aliases = ["mboxes", "mbox"])]
    Mailboxes(ImapMailboxCommand),
    #[command(subcommand)]
    Envelopes(ImapEnvelopeCommand),
    #[command(subcommand)]
    Flags(ImapFlagCommand),
    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(ImapMessageCommand),
}

impl ImapCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Id(cmd) => cmd.execute(printer, account),

            Self::Envelopes(cmd) => cmd.execute(printer, account),
            Self::Flags(cmd) => cmd.execute(printer, account),
            Self::Mailboxes(cmd) => cmd.execute(printer, account),
            Self::Messages(cmd) => cmd.execute(printer, account),
        }
    }
}
