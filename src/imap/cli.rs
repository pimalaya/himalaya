use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
    envelope::{
        search::ImapEnvelopeSearchCommand, sort::ImapEnvelopeSortCommand,
        thread::ImapEnvelopeThreadCommand,
    },
    fetch::ImapFetchCommand,
    flag::{list::ImapFlagListCommand, store::ImapStoreCommand},
    id::ImapIdCommand,
    mailbox::{
        close::ImapMailboxCloseCommand, create::ImapMailboxCreateCommand,
        delete::ImapMailboxDeleteCommand, expunge::ImapMailboxExpungeCommand,
        list::ImapMailboxListCommand, rename::ImapMailboxRenameCommand,
        select::ImapMailboxSelectCommand, status::ImapMailboxStatusCommand,
        subscribe::ImapMailboxSubscribeCommand, unselect::ImapMailboxUnselectCommand,
        unsubscribe::ImapMailboxUnsubscribeCommand,
    },
    message::{
        copy::ImapMessageCopyCommand, r#move::ImapMessageMoveCommand, save::ImapMessageSaveCommand,
    },
    raw::ImapRawCommand,
};

/// IMAP-specific API.
///
/// Gives access to the raw IMAP API. Each command matches the name of
/// its IMAP counterpart (RFC 3501 and extensions), exposed as a flat
/// command list like the protocol itself.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ImapCommand {
    Id(ImapIdCommand),

    // Mailbox lifecycle.
    Select(ImapMailboxSelectCommand),
    Create(ImapMailboxCreateCommand),
    Delete(ImapMailboxDeleteCommand),
    Rename(ImapMailboxRenameCommand),
    Subscribe(ImapMailboxSubscribeCommand),
    Unsubscribe(ImapMailboxUnsubscribeCommand),
    List(ImapMailboxListCommand),
    Status(ImapMailboxStatusCommand),
    Close(ImapMailboxCloseCommand),
    Unselect(ImapMailboxUnselectCommand),
    Expunge(ImapMailboxExpungeCommand),

    // Search and ordering.
    Search(ImapEnvelopeSearchCommand),
    Sort(ImapEnvelopeSortCommand),
    Thread(ImapEnvelopeThreadCommand),

    // Flags.
    Store(ImapStoreCommand),
    Flags(ImapFlagListCommand),

    // Message data.
    Fetch(ImapFetchCommand),
    Append(ImapMessageSaveCommand),
    Copy(ImapMessageCopyCommand),
    Move(ImapMessageMoveCommand),

    // Raw passthrough.
    Raw(ImapRawCommand),
}

impl ImapCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        match self {
            Self::Id(cmd) => cmd.execute(printer, account, client),

            Self::Select(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Subscribe(cmd) => cmd.execute(printer, client),
            Self::Unsubscribe(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Status(cmd) => cmd.execute(printer, account, client),
            Self::Close(cmd) => cmd.execute(printer, client),
            Self::Unselect(cmd) => cmd.execute(printer, client),
            Self::Expunge(cmd) => cmd.execute(printer, client),

            Self::Search(cmd) => cmd.execute(printer, account, client),
            Self::Sort(cmd) => cmd.execute(printer, account, client),
            Self::Thread(cmd) => cmd.execute(printer, client),

            Self::Store(cmd) => cmd.execute(printer, client),
            Self::Flags(cmd) => cmd.execute(printer, account, client),

            Self::Fetch(cmd) => cmd.execute(printer, client),
            Self::Append(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),

            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
