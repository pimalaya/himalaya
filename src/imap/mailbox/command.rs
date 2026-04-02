use anyhow::Result;
use clap::Subcommand;
use pimalaya_toolbox::terminal::printer::Printer;

use crate::imap::{
    account::ImapAccount,
    mailbox::{
        close::ImapMailboxCloseCommand, create::ImapMailboxCreateCommand,
        delete::ImapMailboxDeleteCommand, expunge::ImapMailboxExpungeCommand,
        list::ImapMailboxListCommand, purge::ImapMailboxPurgeCommand,
        rename::ImapMailboxRenameCommand, select::ImapMailboxSelectCommand,
        status::ImapMailboxStatusCommand, subscribe::ImapMailboxSubscribeCommand,
        unselect::ImapMailboxUnselectCommand, unsubscribe::ImapMailboxUnsubscribeCommand,
    },
};

/// Manage IMAP mailboxes.
///
/// A mailbox is a message container. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum ImapMailboxCommand {
    Close(ImapMailboxCloseCommand),
    #[command(alias = "add", alias = "new")]
    Create(ImapMailboxCreateCommand),
    #[command(alias = "remove", alias = "rm")]
    Delete(ImapMailboxDeleteCommand),
    Expunge(ImapMailboxExpungeCommand),
    #[command(alias = "lst")]
    List(ImapMailboxListCommand),
    Purge(ImapMailboxPurgeCommand),
    Rename(ImapMailboxRenameCommand),
    Select(ImapMailboxSelectCommand),
    Status(ImapMailboxStatusCommand),
    Subscribe(ImapMailboxSubscribeCommand),
    Unselect(ImapMailboxUnselectCommand),
    Unsubscribe(ImapMailboxUnsubscribeCommand),
}

impl ImapMailboxCommand {
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
