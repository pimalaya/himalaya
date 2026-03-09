use std::{collections::HashMap, fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::{
        core::Vec1,
        envelope::Address,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use io_stream::runtimes::std::handle;
use log::debug;
use pimalaya_toolbox::terminal::printer::Printer;
use rfc2047_decoder::{Decoder, RecoverStrategy};
use serde::Serialize;

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalArg, MailboxSelectFlag},
    stream,
};

/// List IMAP envelopes from the given mailbox.
///
/// This command displays envelopes for messages in the specified
/// mailbox. You can specify a sequence set to limit which messages
/// are fetched.
#[derive(Debug, Parser)]
pub struct ListEnvelopesCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalArg,
    #[command(flatten)]
    pub select: MailboxSelectFlag,

    /// The sequence set of envelopes.
    #[arg(short, long, default_value = "1:*")]
    pub sequence: String,
    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ListEnvelopesCommand {
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

        let sequence_set: SequenceSet = self.sequence.parse()?;
        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::Envelope]);

        let mut arg = None;
        let mut coroutine = ImapFetch::new(context, sequence_set, item_names, !self.seq);

        let data = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchResult::Ok { data, .. } => break data,
                ImapFetchResult::Err { err, .. } => bail!(err),
            }
        };

        let table = EnvelopesTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            envelopes: map_envelopes_table_entries(!self.seq, data),
            uid_mode: !self.seq,
        };

        printer.out(table)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopesTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    envelopes: Vec<EnvelopesTableEntry>,
    uid_mode: bool,
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        let id_header = if self.uid_mode { "UID" } else { "SEQ" };

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new(id_header),
                Cell::new("Subject"),
                Cell::new("From"),
                Cell::new("Date"),
            ]));

        for entry in &self.envelopes {
            let mut row = Row::new();
            row.max_height(1);
            row.add_cell(Cell::new(entry.id));
            row.add_cell(Cell::new(&entry.subject));
            row.add_cell(Cell::new(&entry.from));
            row.add_cell(Cell::new(&entry.date));
            table.add_row(row);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnvelopesTableEntry {
    pub id: u32,
    pub date: String,
    pub from: String,
    pub subject: String,
}

fn map_envelopes_table_entries(
    uid_mode: bool,
    data: HashMap<NonZeroU32, Vec1<MessageDataItem<'_>>>,
) -> Vec<EnvelopesTableEntry> {
    let mut entries: Vec<EnvelopesTableEntry> = data
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
                            subject = decode_mime(String::from_utf8_lossy(s.as_ref()).as_ref());
                        }
                        from = format_addresses_short(&env.from);
                    }
                    _ => {}
                }
            }

            EnvelopesTableEntry {
                id,
                date,
                from,
                subject,
            }
        })
        .collect();

    entries.sort_by_key(|e| e.id);
    entries.reverse();
    entries
}

/// Decode RFC 2047 MIME-encoded string, falling back to original on error.
pub fn decode_mime(s: &str) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    match decoder.decode(s.as_bytes()) {
        Ok(s) => s,
        Err(err) => {
            debug!("cannot decode rfc2047 string `{s}`: {err}");
            s.to_string()
        }
    }
}

/// Format email address from mailbox and host parts.
fn format_email(addr: &Address<'_>) -> String {
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

    if !mailbox.is_empty() && !host.is_empty() {
        format!("{mailbox}@{host}")
    } else {
        mailbox
    }
}

/// Short format for list view (name OR email, not both).
pub fn format_address_short(addr: &Address<'_>) -> String {
    // If name exists, show decoded name only
    if let Some(n) = &addr.name.0 {
        let name = decode_mime(&String::from_utf8_lossy(n.as_ref()));
        if !name.is_empty() {
            return name;
        }
    }
    // Otherwise show email
    format_email(addr)
}

/// Full format for detailed view (Name <email> or email).
pub fn format_address(addr: &Address<'_>) -> String {
    let email = format_email(addr);
    if let Some(n) = &addr.name.0 {
        let name = decode_mime(&String::from_utf8_lossy(n.as_ref()));
        if !name.is_empty() {
            return format!("{name} <{email}>");
        }
    }
    email
}

/// Short addresses formatter for list view.
pub fn format_addresses_short(addrs: &[Address<'_>]) -> String {
    addrs
        .iter()
        .map(format_address_short)
        .collect::<Vec<_>>()
        .join(", ")
}
