use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient, envelope::cli::ImapEnvelopeCommand, flag::cli::ImapFlagCommand,
    id::ImapIdCommand, mailbox::cli::ImapMailboxCommand, message::cli::ImapMessageCommand,
};

/// IMAP CLI.
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
    pub fn execute(self, printer: &mut impl Printer, client: ImapClient) -> Result<()> {
        match self {
            Self::Id(cmd) => cmd.execute(printer, client),

            Self::Envelopes(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, client),
            Self::Mailboxes(cmd) => cmd.execute(printer, client),
            Self::Messages(cmd) => cmd.execute(printer, client),
        }
    }
}
