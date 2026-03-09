use std::{fmt, num::NonZeroU32};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use io_imap::{
    coroutines::{fetch::*, select::*},
    types::fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
};
use io_stream::runtimes::std::handle;
use mail_parser::{Addr, Address, ContentType, MessageParser, MimeHeaders};
use pimalaya_toolbox::terminal::printer::Printer;
use serde::Serialize;

use crate::{config::ImapConfig, imap::mailbox::arg::MailboxNameOptionalFlag, imap::stream};

/// Get a message and display its structure.
///
/// This command fetches a message and displays its headers along with
/// the body structure tree showing all MIME parts.
#[derive(Debug, Parser)]
pub struct GetMessageCommand {
    #[command(flatten)]
    pub mailbox: MailboxNameOptionalFlag,

    /// The message UID (or sequence number with --seq).
    #[arg(name = "id", value_name = "ID")]
    pub id: u32,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl GetMessageCommand {
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

        // FETCH with BODY.PEEK[] to avoid marking as read
        let id = NonZeroU32::new(self.id).ok_or_else(|| anyhow::anyhow!("ID must be non-zero"))?;

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);

        let mut arg = None;
        let mut coroutine = ImapFetchFirst::new(context, id, item_names, !self.seq);

        let items = loop {
            match coroutine.resume(arg.take()) {
                ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
                ImapFetchFirstResult::Ok { items, .. } => break items,
                ImapFetchFirstResult::Err { err, .. } => bail!(err),
            }
        };

        // Extract raw message bytes
        let mut raw_message: Option<Vec<u8>> = None;
        for item in items.into_iter() {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    raw_message = Some(data.as_ref().to_vec());
                }
            }
        }

        let raw = raw_message.ok_or_else(|| anyhow::anyhow!("No message data returned"))?;

        // Parse message using mail-parser
        let message = MessageParser::default()
            .parse(&raw)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse message"))?;

        let structure = MessageStructure::from_parsed(&message);

        printer.out(structure)?;
        Ok(())
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
    pub body: Option<BodyPart>,
}

impl MessageStructure {
    pub fn from_parsed(message: &mail_parser::Message<'_>) -> Self {
        // Extract headers
        let headers = MessageHeaders {
            date: message.date().map(|d| d.to_rfc3339()),
            subject: message.subject().map(|s| s.to_string()),
            message_id: message.message_id().map(|s| s.to_string()),
            in_reply_to: message.in_reply_to().as_text().map(|s| s.to_string()),
            from: extract_addresses(message.from()),
            to: extract_addresses(message.to()),
            cc: extract_addresses(message.cc()),
            bcc: extract_addresses(message.bcc()),
            reply_to: extract_addresses(message.reply_to()),
        };

        // Build body structure tree
        let body = build_body_tree(message);

        Self { headers, body }
    }
}

fn extract_addresses(addr: Option<&Address<'_>>) -> Vec<String> {
    match addr {
        Some(Address::List(list)) => list.iter().map(format_addr).collect(),
        Some(Address::Group(groups)) => groups
            .iter()
            .flat_map(|g| g.addresses.iter().map(format_addr))
            .collect(),
        None => Vec::new(),
    }
}

fn format_addr(addr: &Addr<'_>) -> String {
    match (&addr.name, &addr.address) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (None, Some(email)) => email.to_string(),
        (Some(name), None) => name.to_string(),
        (None, None) => String::new(),
    }
}

fn format_content_type(ct: &ContentType<'_>) -> String {
    match &ct.c_subtype {
        Some(sub) => format!("{}/{}", ct.c_type, sub),
        None => ct.c_type.to_string(),
    }
}

fn build_body_tree(message: &mail_parser::Message<'_>) -> Option<BodyPart> {
    let content_type = message
        .root_part()
        .content_type()
        .map(format_content_type)
        .unwrap_or_else(|| "text/plain".to_string());

    let is_multipart = content_type.starts_with("multipart/");

    if is_multipart {
        // Multipart message - build tree from parts
        let parts: Vec<BodyPart> = message
            .parts
            .iter()
            .skip(1) // Skip the root part (it's the multipart container)
            .filter_map(|part| build_part_tree(part))
            .collect();

        Some(BodyPart {
            content_type,
            name: None,
            size: None,
            parts,
        })
    } else {
        // Single part message
        let size = message.raw_message.len();
        Some(BodyPart {
            content_type,
            name: None,
            size: Some(size),
            parts: Vec::new(),
        })
    }
}

fn build_part_tree(part: &mail_parser::MessagePart<'_>) -> Option<BodyPart> {
    let content_type = part
        .content_type()
        .map(format_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    // Skip multipart container parts (they're represented by their children)
    if content_type.starts_with("multipart/") {
        return None;
    }

    let name = part.attachment_name().map(|s| s.to_string());
    let size = part.len();

    Some(BodyPart {
        content_type,
        name,
        size: Some(size),
        parts: Vec::new(),
    })
}

fn format_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

impl fmt::Display for MessageStructure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display headers as a table
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

        // Display body structure
        if let Some(body) = &self.body {
            writeln!(f, "\nBody structure:")?;
            write_body_tree(f, body, "", true)?;
        }

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

    // Build the part description
    let mut desc = part.content_type.clone();
    if let Some(name) = &part.name {
        desc.push_str(&format!(" \"{}\"", name));
    }
    if let Some(size) = part.size {
        desc.push_str(&format!(" ({})", format_size(size)));
    }

    writeln!(f, "{}{}{}", prefix, connector, desc)?;

    // Handle children
    let child_prefix = if is_last {
        format!("{}   ", prefix)
    } else {
        format!("{}│  ", prefix)
    };

    for (i, child) in part.parts.iter().enumerate() {
        let is_last_child = i == part.parts.len() - 1;
        write_body_tree(f, child, &child_prefix, is_last_child)?;
    }

    Ok(())
}
