use std::{collections::BTreeMap, fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_imap::types::{
    core::Vec1,
    envelope::Address,
    fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
    sequence::{SeqOrUid, Sequence, SequenceSet},
    status::{StatusDataItem, StatusDataItemName},
};
use log::debug;
use pimalaya_cli::printer::Printer;
use rfc2047_decoder::{Decoder, RecoverStrategy};
use serde::Serialize;

use crate::imap::{
    account::ImapAccount,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// List IMAP envelopes from the given mailbox.
///
/// This command displays envelopes for messages in the specified
/// mailbox. You can specify a sequence set to limit which messages
/// are fetched.
#[derive(Debug, Parser)]
pub struct ImapEnvelopeListCommand {
    /// The sequence set of envelopes.
    #[arg(value_name = "SEQUENCE")]
    #[arg(conflicts_with = "page_size")]
    #[arg(conflicts_with = "page")]
    pub sequence_set: Option<String>,

    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    #[arg(long, default_value = "10")]
    #[arg(conflicts_with = "sequence")]
    pub page_size: usize,
    #[arg(long, short, default_value = "0")]
    #[arg(conflicts_with = "sequence")]
    pub page: usize,

    /// Use sequence numbers instead of UIDs.
    #[arg(long, short, visible_alias = "seq")]
    pub sequence: bool,
}

impl ImapEnvelopeListCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let mailbox = self.mailbox_name.inner.try_into()?;

        let exists = if self.mailbox_no_select.inner {
            let items = client.status(mailbox, &[StatusDataItemName::Messages])?;
            items.into_iter().find_map(|i| match i {
                StatusDataItem::Messages(exists) => Some(exists),
                _ => None,
            })
        } else {
            client.select(mailbox)?.exists
        };

        let mut has_sequence = false;
        let sequence_set = match self.sequence_set {
            Some(seq) => {
                has_sequence = true;
                seq.parse()?
            }
            None => match exists {
                Some(n) => build_paginated_sequence(self.page, self.page_size, n as usize)?,
                None => "1:*".try_into()?,
            },
        };

        let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
            MessageDataItemName::Uid,
            MessageDataItemName::Envelope,
        ]);

        let data = client.fetch(sequence_set, item_names, !self.sequence && has_sequence)?;

        let table = EnvelopesTable {
            preset: account.table_preset,
            arrangement: account.table_arrangement,
            envelopes: map_envelopes_table_entries(data),
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
}

impl fmt::Display for EnvelopesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("SEQ"),
                Cell::new("UID"),
                Cell::new("SUBJECT"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]));

        for entry in &self.envelopes {
            let mut row = Row::new();
            row.max_height(1);
            row.add_cell(Cell::new(entry.seq));
            row.add_cell(Cell::new(entry.uid));
            row.add_cell(Cell::new(&entry.subject));
            row.add_cell(Cell::new(&entry.from));
            row.add_cell(Cell::new(&entry.date));
            table.add_row(row);
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct EnvelopesTableEntry {
    pub seq: u32,
    pub uid: u32,
    pub date: String,
    pub from: String,
    pub subject: String,
}

fn map_envelopes_table_entries(
    data: BTreeMap<NonZeroU32, Vec1<MessageDataItem<'_>>>,
) -> Vec<EnvelopesTableEntry> {
    let mut entries: Vec<EnvelopesTableEntry> = data
        .into_iter()
        .map(|(seq, items)| {
            let mut entry = EnvelopesTableEntry::default();
            entry.seq = seq.get();

            for item in items.into_iter() {
                match item {
                    MessageDataItem::Uid(uid) => {
                        entry.uid = uid.get();
                    }
                    MessageDataItem::Envelope(env) => {
                        if let Some(d) = env.date.into_option() {
                            entry.date = String::from_utf8_lossy(d.as_ref()).to_string();
                        }

                        if let Some(s) = env.subject.into_option() {
                            entry.subject =
                                decode_mime(String::from_utf8_lossy(s.as_ref()).as_ref());
                        }

                        entry.from = format_addresses_short(&env.from);
                    }
                    _ => {}
                }
            }

            entry
        })
        .collect();

    entries.sort_by_key(|e| e.uid);
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

fn build_paginated_sequence(page: usize, page_size: usize, total: usize) -> Result<SequenceSet> {
    let seq = if page_size == 0 {
        Sequence::Range(SeqOrUid::try_from(1).unwrap(), SeqOrUid::Asterisk)
    } else {
        let page_cursor = page * page_size;
        if page_cursor >= total {
            bail!("page {} out of bounds", page + 1);
        }

        let mut count = 1;
        let mut cursor = total - (total.min(page_cursor));

        let page_size = page_size.min(total);
        let from = SeqOrUid::Value(NonZeroU32::new(cursor as u32).unwrap());
        while cursor > 1 && count < page_size {
            count += 1;
            cursor -= 1;
        }
        let to = SeqOrUid::Value(NonZeroU32::new(cursor as u32).unwrap());
        Sequence::Range(to, from)
    };

    Ok(seq.into())
}
