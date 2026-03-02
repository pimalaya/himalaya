// mod add;
// mod delete;
// mod expunge;
pub mod list;
// mod purge;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{account::Account, imap::mailbox::command::list::ListMailboxesCommand};

/// Create, list and purge mailboxes.
///
/// A mailbox is a message container. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    // #[command(visible_alias = "create", alias = "new")]
    // Add(FolderAddCommand),
    List(ListMailboxesCommand),
    // #[command()]
    // Expunge(FolderExpungeCommand),

    // #[command()]
    // Purge(FolderPurgeCommand),

    // #[command(alias = "remove", alias = "rm")]
    // Delete(FolderDeleteCommand),
}

impl MailboxCommand {
    #[allow(unused)]
    pub fn execute(self, printer: &mut impl Printer, account: Account) -> Result<()> {
        match self {
            // Self::Add(cmd) => cmd.execute(printer, config).await,
            Self::List(cmd) => cmd.execute(printer, account),
            // Self::Expunge(cmd) => cmd.execute(printer, config).await,
            // Self::Purge(cmd) => cmd.execute(printer, config).await,
            // Self::Delete(cmd) => cmd.execute(printer, config).await,
        }
    }
}
