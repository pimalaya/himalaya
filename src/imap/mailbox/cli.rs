// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use clap::Subcommand;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::imap::{
    client::ImapClient,
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        match self {
            Self::Close(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
            Self::Expunge(cmd) => cmd.execute(printer, client),
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Purge(cmd) => cmd.execute(printer, client),
            Self::Rename(cmd) => cmd.execute(printer, client),
            Self::Select(cmd) => cmd.execute(printer, client),
            Self::Status(cmd) => cmd.execute(printer, account, client),
            Self::Subscribe(cmd) => cmd.execute(printer, client),
            Self::Unselect(cmd) => cmd.execute(printer, client),
            Self::Unsubscribe(cmd) => cmd.execute(printer, client),
        }
    }
}
