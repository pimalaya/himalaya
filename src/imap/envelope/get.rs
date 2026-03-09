use std::{fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::{
        core::Vec1,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::imap::{
    account::ImapAccount,
    envelope::list::{decode_mime, format_address},
    mailbox::arg::{MailboxNameOptionalFlag, MailboxSelectFlag},
    stream,
};

/// Get a single IMAP envelope.
///
/// This command displays detailed envelope information for a specific
/// message, including all header fields like date, subject, from, to,
/// cc, bcc, reply-to, message-id, and in-reply-to.
#[derive(Debug, Parser)]
pub struct GetEnvelopeCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub select: MailboxSelectFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,
    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl GetEnvelopeCommand {
    pub fn exec(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let (mut context, mut stream) = stream::connect(account.backend)?;

        let mailbox = self.mailbox.name.try_into()?;

        if self.select.r#true {
            let mut arg = None;
            let mut coroutine = ImapSelect::new(context, mailbox);

            context = loop {
                match coroutine.resume(arg.take()) {
                    ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                    ImapSelectResult::Ok { context, .. } => break context,
                    ImapSelectResult::Err { err, .. } => bail!(err),
                }
            };
        }

        let Some(id) = NonZeroU32::new(self.id) else {
            bail!("ID must be non-zero");
        };

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::Envelope]);

        let mut arg = None;
        let mut coroutine = ImapFetchFirst::new(context, id, item_names, !self.seq);

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchFirstResult::Ok { items, .. } => break items,
                ImapFetchFirstResult::Err { err, .. } => bail!(err),
            }
        };

        let table = EnvelopeTable {
            preset: account.table_preset,
            envelope: items.into(),
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeTable {
    #[serde(skip)]
    pub preset: String,
    pub envelope: EnvelopeTableItems,
}

impl fmt::Display for EnvelopeTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("HEADER"), Cell::new("VALUE")]));

        table.add_row(Row::from([
            Cell::new("Message ID"),
            match &self.envelope.message_id {
                Some(id) => Cell::new(id),
                None => Cell::new(""),
            },
        ]));

        table.add_row(Row::from([
            Cell::new("In Reply To"),
            match &self.envelope.in_reply_to {
                Some(id) => Cell::new(id),
                None => Cell::new(""),
            },
        ]));

        table.add_row(Row::from([
            Cell::new("Date"),
            match &self.envelope.date {
                Some(date) => Cell::new(date),
                None => Cell::new(""),
            },
        ]));

        table.add_row(Row::from([
            Cell::new("Subject"),
            match &self.envelope.subject {
                Some(subject) => Cell::new(subject),
                None => Cell::new(""),
            },
        ]));

        table.add_row(Row::from([
            Cell::new("Sender"),
            Cell::new(self.envelope.sender.join(", ")),
        ]));

        table.add_row(Row::from([
            Cell::new("From"),
            Cell::new(self.envelope.from.join(", ")),
        ]));

        table.add_row(Row::from([
            Cell::new("Reply To"),
            Cell::new(self.envelope.reply_to.join(", ")),
        ]));

        table.add_row(Row::from([
            Cell::new("To"),
            Cell::new(self.envelope.to.join(", ")),
        ]));

        table.add_row(Row::from([
            Cell::new("Cc"),
            Cell::new(self.envelope.cc.join(", ")),
        ]));

        table.add_row(Row::from([
            Cell::new("Bcc"),
            Cell::new(self.envelope.bcc.join(", ")),
        ]));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EnvelopeTableItems {
    pub date: Option<String>,
    pub subject: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,

    pub from: Vec<String>,
    pub sender: Vec<String>,
    pub reply_to: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
}

impl From<Vec1<MessageDataItem<'_>>> for EnvelopeTableItems {
    fn from(items: Vec1<MessageDataItem<'_>>) -> Self {
        let mut table = EnvelopeTableItems::default();

        for item in items.into_iter() {
            if let MessageDataItem::Envelope(env) = item {
                if let Some(d) = &env.date.into_option() {
                    table
                        .date
                        .replace(String::from_utf8_lossy(d.as_ref()).to_string());
                }

                if let Some(s) = &env.subject.into_option() {
                    table
                        .subject
                        .replace(decode_mime(&String::from_utf8_lossy(s.as_ref())));
                }

                if let Some(m) = &env.message_id.into_option() {
                    table
                        .message_id
                        .replace(String::from_utf8_lossy(m.as_ref()).to_string());
                }

                if let Some(r) = &env.in_reply_to.into_option() {
                    table
                        .in_reply_to
                        .replace(String::from_utf8_lossy(r.as_ref()).to_string());
                }

                table.from.extend(env.from.iter().map(format_address));
                table.sender.extend(env.sender.iter().map(format_address));
                table
                    .reply_to
                    .extend(env.reply_to.iter().map(format_address));
                table.to.extend(env.to.iter().map(format_address));
                table.cc.extend(env.cc.iter().map(format_address));
                table.bcc.extend(env.bcc.iter().map(format_address));
            }
        }

        table
    }
}
