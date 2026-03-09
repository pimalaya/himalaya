use std::{fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::{
        core::Vec1,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::{
    config::ImapConfig,
    imap::{
        envelope::list::{decode_mime, format_addresses},
        mailbox::arg::MailboxNameOptionalFlag,
        stream,
    },
};

/// Get a single message envelope.
///
/// This command displays detailed envelope information for a specific
/// message, including all header fields like date, subject, from, to,
/// cc, bcc, reply-to, message-id, and in-reply-to.
#[derive(Debug, Parser)]
pub struct GetEnvelopeCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl GetEnvelopeCommand {
    pub fn exec(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailbox = self.mailbox.name.try_into()?;

        // SELECT mailbox
        let mut arg = None;
        let mut coroutine = ImapSelect::new(context, mailbox);

        let context = loop {
            match coroutine.resume(arg.take()) {
                ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapSelectResult::Ok { context, .. } => break context,
                ImapSelectResult::Err { err, .. } => bail!(err),
            }
        };

        // FETCH envelope
        let id = NonZeroU32::new(self.id).ok_or_else(|| anyhow::anyhow!("ID must be non-zero"))?;

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

        let table = EnvelopeDetailTable::new(items);

        printer.out(table)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeDetail {
    pub date: String,
    pub subject: String,
    pub message_id: String,
    pub in_reply_to: String,
    pub from: String,
    pub sender: String,
    pub reply_to: String,
    pub to: String,
    pub cc: String,
    pub bcc: String,
}

pub struct EnvelopeDetailTable {
    detail: EnvelopeDetail,
}

impl EnvelopeDetailTable {
    pub fn new(items: Vec1<MessageDataItem<'static>>) -> Self {
        let mut detail = EnvelopeDetail {
            date: String::new(),
            subject: String::new(),
            message_id: String::new(),
            in_reply_to: String::new(),
            from: String::new(),
            sender: String::new(),
            reply_to: String::new(),
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
        };

        for item in items.into_iter() {
            if let MessageDataItem::Envelope(env) = item {
                // NString wraps Option<IString>, access via .0
                if let Some(d) = &env.date.0 {
                    detail.date = String::from_utf8_lossy(d.as_ref()).to_string();
                }
                if let Some(s) = &env.subject.0 {
                    detail.subject = decode_mime(&String::from_utf8_lossy(s.as_ref()));
                }
                if let Some(m) = &env.message_id.0 {
                    detail.message_id = String::from_utf8_lossy(m.as_ref()).to_string();
                }
                if let Some(r) = &env.in_reply_to.0 {
                    detail.in_reply_to = String::from_utf8_lossy(r.as_ref()).to_string();
                }
                detail.from = format_addresses(&env.from);
                detail.sender = format_addresses(&env.sender);
                detail.reply_to = format_addresses(&env.reply_to);
                detail.to = format_addresses(&env.to);
                detail.cc = format_addresses(&env.cc);
                detail.bcc = format_addresses(&env.bcc);
            }
        }

        Self { detail }
    }
}

impl fmt::Display for EnvelopeDetailTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("FIELD"), Cell::new("VALUE")]));

        let fields = [
            ("Date", &self.detail.date),
            ("Subject", &self.detail.subject),
            ("Message-ID", &self.detail.message_id),
            ("From", &self.detail.from),
            ("Sender", &self.detail.sender),
            ("To", &self.detail.to),
            ("Cc", &self.detail.cc),
            ("Bcc", &self.detail.bcc),
            ("Reply-To", &self.detail.reply_to),
            ("In-Reply-To", &self.detail.in_reply_to),
        ];

        for (name, value) in fields {
            table.add_row(Row::from([Cell::new(name), Cell::new(value)]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for EnvelopeDetailTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.detail.serialize(serializer)
    }
}
