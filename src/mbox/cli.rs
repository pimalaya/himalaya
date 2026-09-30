//! # mbox command
//!
//! The `mbox` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    mbox::{
        client::MboxClient, create::MboxMailboxCreateCommand, delete::MboxMailboxDeleteCommand,
        flag::cli::MboxFlagCommand, list::MboxMailboxListCommand, message::cli::MboxMessageCommand,
        rename::MboxMailboxRenameCommand,
    },
};

/// mbox-specific API.
///
/// This command gives you access to the raw mbox API: one file per
/// mailbox, flags stored in the `Status`, `X-Status` and `X-Keywords`
/// headers.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum MboxCommand {
    Create(MboxMailboxCreateCommand),
    Rename(MboxMailboxRenameCommand),
    Delete(MboxMailboxDeleteCommand),
    List(MboxMailboxListCommand),
    #[command(subcommand)]
    #[command(visible_alias = "msg", aliases = ["messages", "msgs"])]
    Message(MboxMessageCommand),
    #[command(subcommand)]
    #[command(alias = "flags")]
    Flag(MboxFlagCommand),
}

impl MboxCommand {
    /// Runs the subcommand against the account's mbox files.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MboxClient,
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
