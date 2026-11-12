//! # Mailbox command
//!
//! The `mailbox` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    capability::{self, Capability},
    shared::{
        client::EmailClient,
        mailbox::{
            create::MailboxCreateCommand, delete::MailboxDeleteCommand, list::MailboxListCommand,
            rename::MailboxRenameCommand,
        },
    },
};

/// Manage mailboxes using the unified API.
///
/// A mailbox is a message container.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    #[command(visible_alias = "ls", after_long_help = capability::help("mailbox", "list"))]
    List(MailboxListCommand),
    #[command(visible_alias = "new", after_long_help = capability::help("mailbox", "create"))]
    Create(MailboxCreateCommand),
    #[command(visible_alias = "mv", after_long_help = capability::help("mailbox", "rename"))]
    Rename(MailboxRenameCommand),
    #[command(visible_alias = "rm", after_long_help = capability::help("mailbox", "delete"))]
    Delete(MailboxDeleteCommand),
}

impl MailboxCommand {
    /// The capability the subcommand reaches, checked before any
    /// connection.
    pub fn capabilities(&self) -> Vec<Capability> {
        let capability = match self {
            Self::List(_) => Capability::MailboxList,
            Self::Create(_) => Capability::MailboxCreate,
            Self::Rename(_) => Capability::MailboxRename,
            Self::Delete(_) => Capability::MailboxDelete,
        };

        vec![capability]
    }

    /// Runs the subcommand against the active account's client.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Create(cmd) => cmd.execute(printer, account, client),
            Self::Rename(cmd) => cmd.execute(printer, account, client),
            Self::Delete(cmd) => cmd.execute(printer, account, client),
        }
    }
}
