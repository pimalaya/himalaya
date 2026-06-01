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
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::shared::{client::EmailClient, flags::arg::MessageIdsArg};

/// Move message(s) from one mailbox to another within the active
/// account.
///
/// IMAP uses `UID MOVE` (RFC 6851); JMAP uses `Email/set` patches that
/// remove the source and add the destination from each email's
/// `mailboxIds`; Maildir renames the underlying file. Cross-account /
/// cross-backend move is out of scope.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    /// Source mailbox name or path (IMAP/Maildir). For JMAP this is
    /// resolved by exact-match name against `Mailbox/get`.
    #[arg(
        long = "from",
        short = 'f',
        value_name = "NAME",
        default_value = "Inbox"
    )]
    pub from: String,

    /// Destination mailbox name or path. Mandatory.
    #[arg(long = "to", short = 't', value_name = "NAME")]
    pub to: String,
}

impl MessageMoveCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut EmailClient) -> Result<()> {
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        client.move_messages(&self.from, &self.to, &ids)?;
        printer.out(Message::new("Message(s) successfully moved"))
    }
}
