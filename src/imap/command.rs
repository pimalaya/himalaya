use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount, envelope::command::EnvelopeCommand, flag::command::FlagCommand,
    id::IdCommand, mailbox::command::MailboxCommand, message::command::MessageCommand,
};

/// IMAP CLI (requires the `imap` cargo feature).
///
/// This command gives you access to the IMAP CLI API, and allows you
/// to manage IMAP mailboxes, envelopes, flags, messages etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "lowercase")]
pub enum ImapCommand {
    Id(IdCommand),

    #[command(subcommand)]
    #[command(aliases = ["mboxes", "mbox"])]
    Mailboxes(MailboxCommand),
    #[command(subcommand)]
    Envelopes(EnvelopeCommand),
    #[command(subcommand)]
    Flags(FlagCommand),
    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(MessageCommand),
}

impl ImapCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Id(cmd) => cmd.exec(printer, account),

            Self::Envelopes(cmd) => cmd.exec(printer, account),
            Self::Flags(cmd) => cmd.exec(printer, account),
            Self::Mailboxes(cmd) => cmd.exec(printer, account),
            Self::Messages(cmd) => cmd.exec(printer, account),
        }
    }
}
