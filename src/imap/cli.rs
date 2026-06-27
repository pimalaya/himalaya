use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient, envelope::cli::ImapEnvelopeCommand, flag::cli::ImapFlagCommand,
    id::ImapIdCommand, mailbox::cli::ImapMailboxCommand, message::cli::ImapMessageCommand,
};

/// IMAP-specific API.
///
/// Gives access to the raw IMAP API. Every CLI command matches the name of its
/// IMAP counterpart, grouped by domain: mailbox, envelope, flag and message.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ImapCommand {
    Id(ImapIdCommand),

    #[command(subcommand)]
    #[command(aliases = ["mbox"])]
    Mailbox(ImapMailboxCommand),
    #[command(subcommand)]
    Envelope(ImapEnvelopeCommand),
    #[command(subcommand)]
    Flag(ImapFlagCommand),
    #[command(subcommand)]
    #[command(aliases = ["msg"])]
    Message(ImapMessageCommand),
}

impl ImapCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        match self {
            Self::Id(cmd) => cmd.execute(printer, account, client),

            Self::Envelope(cmd) => cmd.execute(printer, account, client),
            Self::Flag(cmd) => cmd.execute(printer, account, client),
            Self::Mailbox(cmd) => cmd.execute(printer, account, client),
            Self::Message(cmd) => cmd.execute(printer, client),
        }
    }
}
