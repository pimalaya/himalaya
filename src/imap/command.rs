use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::{
        envelope::command::EnvelopeCommand, flag::command::FlagCommand,
        mailbox::command::MailboxCommand,
    },
};

/// IMAP CLI (requires `imap` cargo feature).
///
/// This command gives you access to the IMAP CLI API, and allows
/// you to manage IMAP mailboxes: list mailboxes, read messages,
/// add flags etc.
#[derive(Debug, Subcommand)]
#[command(rename_all = "lowercase")]
pub enum ImapCommand {
    #[command(subcommand)]
    #[command(aliases = ["envelope", "env"])]
    Envelopes(EnvelopeCommand),
    #[command(subcommand)]
    Flags(FlagCommand),
    #[command(subcommand)]
    #[command(aliases = ["mboxes", "mbox"])]
    Mailboxes(MailboxCommand),
}

impl ImapCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Envelopes(cmd) => cmd.execute(printer, config),
            Self::Flags(cmd) => cmd.execute(printer, config),
            Self::Mailboxes(cmd) => cmd.execute(printer, config),
        }
    }
}
