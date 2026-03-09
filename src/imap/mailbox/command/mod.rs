pub mod close;
pub mod create;
pub mod delete;
pub mod expunge;
pub mod list;
pub mod purge;
pub mod rename;
pub mod select;
pub mod status;
pub mod subscribe;
pub mod unselect;
pub mod unsubscribe;

use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::mailbox::command::{
        close::CloseMailboxCommand, create::CreateMailboxCommand, delete::DeleteMailboxCommand,
        expunge::ExpungeMailboxCommand, list::ListMailboxesCommand, purge::PurgeMailboxCommand,
        rename::RenameMailboxCommand, select::SelectMailboxCommand, status::StatusMailboxCommand,
        subscribe::SubscribeMailboxCommand, unselect::UnselectMailboxCommand,
        unsubscribe::UnsubscribeMailboxCommand,
    },
};

/// Manage IMAP mailboxes.
///
/// A mailbox is a message container. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum MailboxCommand {
    Close(CloseMailboxCommand),
    #[command(alias = "add", alias = "new")]
    Create(CreateMailboxCommand),
    #[command(alias = "remove", alias = "rm")]
    Delete(DeleteMailboxCommand),
    Expunge(ExpungeMailboxCommand),
    #[command(alias = "lst")]
    List(ListMailboxesCommand),
    Purge(PurgeMailboxCommand),
    Rename(RenameMailboxCommand),
    Select(SelectMailboxCommand),
    Status(StatusMailboxCommand),
    Subscribe(SubscribeMailboxCommand),
    Unselect(UnselectMailboxCommand),
    Unsubscribe(UnsubscribeMailboxCommand),
}

impl MailboxCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        match self {
            Self::Close(cmd) => cmd.execute(printer, config),
            Self::Create(cmd) => cmd.execute(printer, config),
            Self::Delete(cmd) => cmd.execute(printer, config),
            Self::Expunge(cmd) => cmd.execute(printer, config),
            Self::List(cmd) => cmd.execute(printer, config),
            Self::Purge(cmd) => cmd.execute(printer, config),
            Self::Rename(cmd) => cmd.execute(printer, config),
            Self::Select(cmd) => cmd.execute(printer, config),
            Self::Status(cmd) => cmd.execute(printer, config),
            Self::Subscribe(cmd) => cmd.execute(printer, config),
            Self::Unselect(cmd) => cmd.execute(printer, config),
            Self::Unsubscribe(cmd) => cmd.execute(printer, config),
        }
    }
}
