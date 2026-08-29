//! # Mailbox command
//!
//! The `mailbox` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{client::EmailClient, mailbox::list::MailboxListCommand},
};

/// Manage mailboxes using the shared API.
///
/// A mailbox is a message container.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    #[command(visible_alias = "ls")]
    List(MailboxListCommand),
}

impl MailboxCommand {
    /// Runs the subcommand against the active account's client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
        }
    }
}
