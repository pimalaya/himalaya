pub mod create;
pub mod delete;
pub mod expunge;
pub mod list;
pub mod purge;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::mailbox::command::{
        create::CreateMailboxCommand, delete::DeleteMailboxCommand,
        expunge::ExpungeMailboxCommand, list::ListMailboxesCommand, purge::PurgeMailboxCommand,
    },
};

/// Create, list and purge mailboxes.
///
/// A mailbox is a message container. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    #[command(alias = "add", alias = "new")]
    Create(CreateMailboxCommand),
    #[command(alias = "remove", alias = "rm")]
    Delete(DeleteMailboxCommand),
    Expunge(ExpungeMailboxCommand),
    List(ListMailboxesCommand),
    Purge(PurgeMailboxCommand),
}

impl MailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Create(cmd) => cmd.execute(printer, config),
            Self::Delete(cmd) => cmd.execute(printer, config),
            Self::Expunge(cmd) => cmd.execute(printer, config),
            Self::List(cmd) => cmd.execute(printer, config),
            Self::Purge(cmd) => cmd.execute(printer, config),
        }
    }
}
