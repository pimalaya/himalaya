use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::{
    config::ImapConfig,
    imap::{
        account::ImapAccount,
        mailbox::{
            close::CloseMailboxCommand, create::CreateMailboxCommand, delete::DeleteMailboxCommand,
            expunge::ExpungeMailboxCommand, list::ListMailboxesCommand, purge::PurgeMailboxCommand,
            rename::RenameMailboxCommand, select::SelectMailboxCommand,
            status::StatusMailboxCommand, subscribe::SubscribeMailboxCommand,
            unselect::UnselectMailboxCommand, unsubscribe::UnsubscribeMailboxCommand,
        },
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
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        match self {
            Self::Close(cmd) => cmd.exec(printer, account),
            Self::Create(cmd) => cmd.exec(printer, account),
            Self::Delete(cmd) => cmd.exec(printer, account),
            Self::Expunge(cmd) => cmd.exec(printer, account),
            Self::List(cmd) => cmd.exec(printer, account),
            Self::Purge(cmd) => cmd.exec(printer, account),
            Self::Rename(cmd) => cmd.exec(printer, account),
            Self::Select(cmd) => cmd.exec(printer, account),
            Self::Status(cmd) => cmd.exec(printer, account),
            Self::Subscribe(cmd) => cmd.exec(printer, account),
            Self::Unselect(cmd) => cmd.exec(printer, account),
            Self::Unsubscribe(cmd) => cmd.exec(printer, account),
        }
    }
}
