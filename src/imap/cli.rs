//! # IMAP command
//!
//! The `imap` command, dispatching onto its subcommands.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    imap::{
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
            copy::ImapMessageCopyCommand, r#move::ImapMessageMoveCommand,
            save::ImapMessageSaveCommand,
        },
        raw::ImapRawCommand,
    },
};

/// IMAP-specific API.
///
/// Each subcommand carries the name of its RFC 3501 counterpart, laid out
/// as the flat command list the protocol itself is.
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum ImapCommand {
    Id(ImapIdCommand),
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
    Search(ImapEnvelopeSearchCommand),
    Sort(ImapEnvelopeSortCommand),
    Thread(ImapEnvelopeThreadCommand),
    Store(ImapStoreCommand),
    #[command(alias = "flags")]
    Flag(ImapFlagListCommand),
    Fetch(ImapFetchCommand),
    Append(ImapMessageSaveCommand),
    Copy(ImapMessageCopyCommand),
    Move(ImapMessageMoveCommand),
    Raw(ImapRawCommand),
}

impl ImapCommand {
    /// Runs the subcommand against the account's IMAP session.
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
            Self::Flag(cmd) => cmd.execute(printer, account, client),

            Self::Fetch(cmd) => cmd.execute(printer, client),
            Self::Append(cmd) => cmd.execute(printer, client),
            Self::Copy(cmd) => cmd.execute(printer, client),
            Self::Move(cmd) => cmd.execute(printer, client),

            Self::Raw(cmd) => cmd.execute(printer, client),
        }
    }
}
