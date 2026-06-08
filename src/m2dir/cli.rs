use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::m2dir::{
    client::M2dirClient, create::M2dirMailboxCreateCommand, delete::M2dirMailboxDeleteCommand,
    envelope::cli::M2dirEnvelopeCommand, flag::cli::M2dirFlagCommand,
    list::M2dirMailboxListCommand, message::cli::M2dirMessageCommand,
};

/// m2dir CLI.
///
/// Protocol-specific entry point for the m2dir backend. Mailbox and
/// per-folder operations (messages, flags, envelopes) live here;
/// cross-backend shared commands also dispatch here when
/// `--backend m2dir` is passed.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum M2dirCommand {
    Create(M2dirMailboxCreateCommand),
    Delete(M2dirMailboxDeleteCommand),
    List(M2dirMailboxListCommand),

    #[command(subcommand)]
    #[command(aliases = ["msgs", "msg"])]
    Messages(M2dirMessageCommand),
    #[command(subcommand)]
    Flags(M2dirFlagCommand),
    #[command(subcommand)]
    Envelopes(M2dirEnvelopeCommand),
}

impl M2dirCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, account, client),

            Self::Messages(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, account, client),
            Self::Envelopes(cmd) => cmd.execute(printer, account, client),
        }
    }
}
