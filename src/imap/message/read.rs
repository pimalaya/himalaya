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

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_imap::types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName};
use mail_parser::{Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Read message content.
///
/// This command fetches a message and displays its text content.
/// By default it shows plain text content; use --html to show HTML.
#[derive(Debug, Parser)]
pub struct ImapMessageReadCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: String,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,

    /// Show HTML content instead of plain text.
    #[arg(long)]
    pub html: bool,
}

impl ImapMessageReadCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox)?;
        }

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);

        let sequence_set = self.id.parse()?;
        let mut data = client.fetch(sequence_set, item_names, !self.seq)?;

        let Some((_, items)) = data.pop_first() else {
            bail!("Read message `{}` error: no message data returned", self.id);
        };

        let mut raw_message: Option<Vec<u8>> = None;

        for item in items.into_iter() {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    raw_message = Some(data.as_ref().to_vec());
                }
            }
        }

        let Some(raw) = raw_message else {
            bail!("Read message `{}` error: no message data returned", self.id);
        };

        let Some(message) = MessageParser::new().parse(&raw) else {
            bail!(
                "Read message `{}` error: failed to parse MIME message",
                self.id
            );
        };

        if self.html {
            printer.out(MessageHtmlView { message })
        } else {
            printer.out(MessagePlainView { message })
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessagePlainView<'a> {
    message: Message<'a>,
}

impl fmt::Display for MessagePlainView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.message.text_bodies().enumerate() {
            if i > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            if let Some(contents) = part.text_contents() {
                write!(f, "{}", contents.trim_end())?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageHtmlView<'a> {
    message: Message<'a>,
}

impl fmt::Display for MessageHtmlView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.message.html_bodies().enumerate() {
            if i > 0 {
                writeln!(f)?;
                writeln!(f)?;
            }

            if let Some(contents) = part.text_contents() {
                write!(f, "{}", contents.trim_end())?;
            }
        }

        Ok(())
    }
}
