use std::fmt;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::{
        core::Vec1,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::{config::ImapConfig, imap::mailbox::arg::name::MailboxNameOptionalArg, imap::stream};

/// List message envelopes in a mailbox.
///
/// This command displays envelopes for messages in the specified
/// mailbox. You can specify a sequence set to limit which messages
/// are fetched.
#[derive(Debug, Parser)]
pub struct ListEnvelopesCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalArg,

    /// The sequence set of messages (default: "1:*" for all).
    #[arg(short, long, default_value = "1:*")]
    pub sequence: String,

    /// Use UID FETCH instead of FETCH.
    #[arg(long)]
    pub uid: bool,
}

impl ListEnvelopesCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
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

        // Parse sequence set
        let sequence_set: SequenceSet = self.sequence.parse()?;

        // FETCH envelopes
        let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
            MessageDataItemName::Envelope,
        ]);

        let mut arg = None;
        let mut coroutine = ImapFetch::new(context, sequence_set, item_names, self.uid);

        let data = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchResult::Ok { data, .. } => break data,
                ImapFetchResult::Err { err, .. } => bail!(err),
            }
        };

        let table = EnvelopesTable::new(data, self.uid);

        printer.out(table)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopeEntry {
    pub id: u32,
    pub date: String,
    pub from: String,
    pub subject: String,
}

pub struct EnvelopesTable {
    entries: Vec<EnvelopeEntry>,
    uid_mode: bool,
}

impl EnvelopesTable {
    pub fn new(
        data: std::collections::HashMap<std::num::NonZeroU32, Vec1<MessageDataItem<'static>>>,
        uid_mode: bool,
    ) -> Self {
        let mut entries: Vec<EnvelopeEntry> = data
            .into_iter()
            .map(|(seq, items)| {
                let mut id = seq.get();
                let mut date = String::new();
                let mut from = String::new();
                let mut subject = String::new();

                for item in items.into_iter() {
                    match item {
                        MessageDataItem::Uid(uid) => {
                            if uid_mode {
                                id = uid.get();
                            }
                        }
                        MessageDataItem::Envelope(env) => {
                            // NString wraps Option<IString>, access via .0
                            if let Some(d) = &env.date.0 {
                                date = String::from_utf8_lossy(d.as_ref()).to_string();
                            }
                            if let Some(s) = &env.subject.0 {
                                subject = String::from_utf8_lossy(s.as_ref()).to_string();
                            }
                            from = format_addresses(&env.from);
                        }
                        _ => {}
                    }
                }

                EnvelopeEntry {
                    id,
                    date,
                    from,
                    subject,
                }
            })
            .collect();

        entries.sort_by_key(|e| e.id);

        Self {
            entries,
            uid_mode,
        }
    }
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new(id_header),
                Cell::new("DATE"),
                Cell::new("FROM"),
                Cell::new("SUBJECT"),
            ]));

        for entry in &self.entries {
            let mut row = Row::new();
            row.max_height(1);
            row.add_cell(Cell::new(entry.id));
            row.add_cell(Cell::new(&entry.date));
            row.add_cell(Cell::new(&entry.from));
            row.add_cell(Cell::new(&entry.subject));
            table.add_row(row);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for EnvelopesTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.entries.serialize(serializer)
    }
}

use io_imap::types::envelope::Address;

pub fn format_address(addr: &Address<'_>) -> String {
    // NString wraps Option<IString>, access via .0
    let mailbox = addr
        .mailbox
        .0
        .as_ref()
        .map(|m| String::from_utf8_lossy(m.as_ref()).to_string())
        .unwrap_or_default();
    let host = addr
        .host
        .0
        .as_ref()
        .map(|h| String::from_utf8_lossy(h.as_ref()).to_string())
        .unwrap_or_default();
    let name = addr
        .name
        .0
        .as_ref()
        .map(|n| String::from_utf8_lossy(n.as_ref()).to_string());

    let email = if !mailbox.is_empty() && !host.is_empty() {
        format!("{mailbox}@{host}")
    } else {
        mailbox
    };

    match name {
        Some(n) if !n.is_empty() => format!("{n} <{email}>"),
        _ => email,
    }
}

pub fn format_addresses(addrs: &[Address<'_>]) -> String {
    addrs
        .iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ")
}
