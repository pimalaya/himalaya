use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{client::EmailClient, mailbox::list::MailboxListCommand};

/// Manage mailboxes using the shared API.
///
/// A mailbox is a message container.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    #[command(visible_alias = "ls")]
    List(MailboxListCommand),
}

impl MailboxCommand {
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
