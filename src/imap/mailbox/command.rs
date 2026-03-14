use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    mailbox::{
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
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Close(cmd) => cmd.execute(printer, account),
            Self::Create(cmd) => cmd.execute(printer, account),
            Self::Delete(cmd) => cmd.execute(printer, account),
            Self::Expunge(cmd) => cmd.execute(printer, account),
            Self::List(cmd) => cmd.execute(printer, account),
            Self::Purge(cmd) => cmd.execute(printer, account),
            Self::Rename(cmd) => cmd.execute(printer, account),
            Self::Select(cmd) => cmd.execute(printer, account),
            Self::Status(cmd) => cmd.execute(printer, account),
            Self::Subscribe(cmd) => cmd.execute(printer, account),
            Self::Unselect(cmd) => cmd.execute(printer, account),
            Self::Unsubscribe(cmd) => cmd.execute(printer, account),
        }
    }
}
