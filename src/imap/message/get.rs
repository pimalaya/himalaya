use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table, presets};
use io_imap::{
    rfc3501::{fetch::ImapMessageFetchOptions, select::ImapMailboxSelectOptions},
    types::{
        body::{BasicFields, BodyStructure, SpecificFields},
        core::{IString, NString},
        envelope::{Address, Envelope},
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
    },
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Get an IMAP message structure (FETCH ENVELOPE BODYSTRUCTURE).
///
/// Displays the envelope headers and the server-reported MIME body
/// structure tree (types, sizes, names), without downloading the
/// message body. To read a body or extract parts, use the shared
/// `messages` and `attachments` commands.
#[derive(Debug, Parser)]
pub struct ImapMessageGetCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The message UID (or sequence number with --seq).
    pub id: String,
    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
            MessageDataItemName::Envelope,
            MessageDataItemName::BodyStructure,
        ]);

        let sequence_set = self.id.parse()?;
        let mut data = client.fetch(
            sequence_set,
            item_names,
            ImapMessageFetchOptions {
                uid: !self.seq,
                modifiers: Vec::new(),
            },
        )?;

        let Some((_, items)) = data.pop_first() else {
            bail!("Get message `{}` error: no message data returned", self.id);
        };

        let mut envelope = None;
        let mut body_structure = None;
        for item in items.into_iter() {
            match item {
                MessageDataItem::Envelope(env) => envelope = Some(env),
                MessageDataItem::BodyStructure(bs) => body_structure = Some(bs),
                _ => {}
            }
        }

        let (Some(envelope), Some(body_structure)) = (envelope, body_structure) else {
            bail!(
                "Get message `{}` error: missing envelope or body structure",
                self.id
            );
        };

        let structure = MessageStructure::from_fetch(&envelope, &body_structure);
        printer.out(structure)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageHeaders {
    pub date: Option<String>,
    pub subject: Option<String>,
    pub message_id: Option<String>,
    pub in_reply_to: Option<String>,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BodyPart {
    pub content_type: String,
    pub name: Option<String>,
    pub size: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<BodyPart>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageStructure {
    pub headers: MessageHeaders,
    pub body: BodyPart,
}

impl MessageStructure {
    pub fn from_fetch(envelope: &Envelope<'_>, body_structure: &BodyStructure<'_>) -> Self {
        let headers = MessageHeaders {
            date: nstring(&envelope.date),
            subject: nstring(&envelope.subject),
            message_id: nstring(&envelope.message_id),
            in_reply_to: nstring(&envelope.in_reply_to),
            from: format_addresses(&envelope.from),
            to: format_addresses(&envelope.to),
            cc: format_addresses(&envelope.cc),
            bcc: format_addresses(&envelope.bcc),
            reply_to: format_addresses(&envelope.reply_to),
        };

        Self {
            headers,
            body: build_part(body_structure),
        }
    }
}

/// Maps a server body structure node to a display part, recursing into
/// multipart children and message/rfc822 encapsulated structures.
fn build_part(structure: &BodyStructure<'_>) -> BodyPart {
    match structure {
        BodyStructure::Single { body, .. } => {
            let parts = match &body.specific {
                SpecificFields::Message { body_structure, .. } => vec![build_part(body_structure)],
                _ => Vec::new(),
            };

            BodyPart {
                content_type: content_type(&body.specific),
                name: part_name(&body.basic),
                size: Some(body.basic.size as usize),
                parts,
            }
        }
        BodyStructure::Multi {
            bodies, subtype, ..
        } => BodyPart {
            content_type: format!("multipart/{}", istring(subtype)),
            name: None,
            size: None,
            parts: bodies.as_ref().iter().map(build_part).collect(),
        },
    }
}

fn content_type(specific: &SpecificFields<'_>) -> String {
    match specific {
        SpecificFields::Basic { r#type, subtype } => {
            format!("{}/{}", istring(r#type), istring(subtype))
        }
        SpecificFields::Message { .. } => "message/rfc822".to_string(),
        SpecificFields::Text { subtype, .. } => format!("text/{}", istring(subtype)),
    }
}

fn part_name(basic: &BasicFields<'_>) -> Option<String> {
    basic
        .parameter_list
        .iter()
        .find(|(key, _)| istring(key).eq_ignore_ascii_case("name"))
        .map(|(_, value)| istring(value))
}

fn format_addresses(addresses: &[Address<'_>]) -> Vec<String> {
    addresses
        .iter()
        .map(format_address)
        .filter(|addr| !addr.is_empty())
        .collect()
}

fn format_address(addr: &Address<'_>) -> String {
    let email = match (nstring(&addr.mailbox), nstring(&addr.host)) {
        (Some(mailbox), Some(host)) => Some(format!("{mailbox}@{host}")),
        (Some(mailbox), None) => Some(mailbox),
        _ => None,
    };

    match (nstring(&addr.name), email) {
        (Some(name), Some(email)) => format!("{name} <{email}>"),
        (None, Some(email)) => email,
        (Some(name), None) => name,
        (None, None) => String::new(),
    }
}

fn istring(string: &IString<'_>) -> String {
    String::from_utf8_lossy(string.as_ref()).into_owned()
}

fn nstring(string: &NString<'_>) -> Option<String> {
    string
        .0
        .as_ref()
        .map(|inner| String::from_utf8_lossy(inner.as_ref()).into_owned())
}

fn format_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

impl fmt::Display for MessageStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([Cell::new("HEADER"), Cell::new("VALUE")]));

        if let Some(date) = &self.headers.date {
            table.add_row(Row::from([Cell::new("Date"), Cell::new(date)]));
        }
        if let Some(subject) = &self.headers.subject {
            table.add_row(Row::from([Cell::new("Subject"), Cell::new(subject)]));
        }
        if let Some(message_id) = &self.headers.message_id {
            table.add_row(Row::from([Cell::new("Message-ID"), Cell::new(message_id)]));
        }
        if !self.headers.from.is_empty() {
            table.add_row(Row::from([
                Cell::new("From"),
                Cell::new(self.headers.from.join(", ")),
            ]));
        }
        if !self.headers.to.is_empty() {
            table.add_row(Row::from([
                Cell::new("To"),
                Cell::new(self.headers.to.join(", ")),
            ]));
        }
        if !self.headers.cc.is_empty() {
            table.add_row(Row::from([
                Cell::new("Cc"),
                Cell::new(self.headers.cc.join(", ")),
            ]));
        }
        if !self.headers.bcc.is_empty() {
            table.add_row(Row::from([
                Cell::new("Bcc"),
                Cell::new(self.headers.bcc.join(", ")),
            ]));
        }
        if !self.headers.reply_to.is_empty() {
            table.add_row(Row::from([
                Cell::new("Reply-To"),
                Cell::new(self.headers.reply_to.join(", ")),
            ]));
        }
        if let Some(in_reply_to) = &self.headers.in_reply_to {
            table.add_row(Row::from([
                Cell::new("In-Reply-To"),
                Cell::new(in_reply_to),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;

        writeln!(f, "\nBody structure:")?;
        write_body_tree(f, &self.body, "", true)?;

        writeln!(f)?;
        Ok(())
    }
}

fn write_body_tree(
    f: &mut fmt::Formatter<'_>,
    part: &BodyPart,
    prefix: &str,
    is_last: bool,
) -> fmt::Result {
    let connector = if is_last { "└─ " } else { "├─ " };

    let mut desc = part.content_type.clone();
    if let Some(name) = &part.name {
        desc.push_str(&format!(" \"{name}\""));
    }
    if let Some(size) = part.size {
        desc.push_str(&format!(" ({})", format_size(size)));
    }

    writeln!(f, "{prefix}{connector}{desc}")?;

    let child_prefix = if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}│  ")
    };

    for (i, child) in part.parts.iter().enumerate() {
        let is_last_child = i == part.parts.len() - 1;
        write_body_tree(f, child, &child_prefix, is_last_child)?;
    }

    Ok(())
}
