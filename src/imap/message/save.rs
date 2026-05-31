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
use io_imap::types::{
    IntoStatic, core::Literal, extensions::binary::LiteralOrLiteral8, flag::Flag, mailbox::Mailbox,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg},
    shared::messages::arg::MessageArg,
};

/// Save a message to a mailbox.
///
/// Appends a message to the specified mailbox. The message can be
/// passed as a positional file path, an inline raw string, or piped
/// via stdin (see [`MessageArg`] for resolution order).
#[derive(Debug, Parser)]
pub struct ImapMessageSaveCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameArg,

    /// The flags to add to the message.
    #[arg(short, long, num_args = 0..)]
    pub flag: Vec<String>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl ImapMessageSaveCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: ImapClient) -> Result<()> {
        let mailbox: Mailbox<'static> = self.mailbox.inner.try_into()?;
        let message = self.message.parse()?;
        let message = Literal::try_from(message)?;
        let message = LiteralOrLiteral8::Literal(message);

        let flags: Vec<_> = self
            .flag
            .iter()
            .map(String::as_str)
            .map(|f| Flag::try_from(f).map(IntoStatic::into_static))
            .collect::<Result<_, _>>()?;

        client.append(mailbox, flags, None, message)?;

        printer.out(Message::new("Message successfully saved"))
    }
}
