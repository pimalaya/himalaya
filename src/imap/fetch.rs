//! # IMAP fetch
//!
//! The `imap fetch` command, RFC 3501 `FETCH`.

use io_imap::client::ImapClient as _;
use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_imap::{
    rfc3501::{fetch::ImapMessageFetchOptions, select::ImapMailboxSelectOptions},
    types::{
        body::{BasicFields, BodyStructure, SpecificFields},
        core::{IString, NString},
        envelope::Envelope,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::FlagFetch,
    },
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
    utils::{decode_mime, format_address},
};

/// Fetch message data items (FETCH, RFC 3501).
///
/// One block is printed per message. The flags below pick the items, and
/// with none `--envelope` is assumed. The UID is always fetched.
#[derive(Debug, Parser)]
pub struct ImapFetchCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    /// The messages to fetch, as `1`, `1,2,3` or `1:*`.
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,
    /// Fetch the envelope: date, subject and addresses.
    #[arg(long)]
    pub envelope: bool,
    /// Fetch the MIME body structure tree.
    #[arg(long)]
    pub structure: bool,
    /// Fetch the flags set on the message.
    #[arg(long)]
    pub flags: bool,
    /// Fetch the internal (server) date.
    #[arg(long)]
    pub internal_date: bool,
    /// Fetch the size, in octets.
    #[arg(long)]
    pub size: bool,
    /// Read the sequence set as message numbers rather than UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapFetchCommand {
    /// Selects the mailbox unless told not to, then fetches the items.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let any = self.envelope || self.structure || self.flags || self.internal_date || self.size;
        let want_envelope = self.envelope || !any;

        let mut names = vec![MessageDataItemName::Uid];
        if want_envelope {
            names.push(MessageDataItemName::Envelope);
        }
        if self.structure {
            names.push(MessageDataItemName::BodyStructure);
        }
        if self.flags {
            names.push(MessageDataItemName::Flags);
        }
        if self.internal_date {
            names.push(MessageDataItemName::InternalDate);
        }
        if self.size {
            names.push(MessageDataItemName::Rfc822Size);
        }

        let sequence_set = self.sequence_set.parse()?;
        let data = client.fetch(
            sequence_set,
            MacroOrMessageDataItemNames::MessageDataItemNames(names),
            ImapMessageFetchOptions {
                uid: !self.seq,
                modifiers: Vec::new(),
            },
        )?;

        let messages = data
            .into_iter()
            .map(|(seq, items)| FetchedMessage::from_items(seq.get(), items.into_iter()))
            .collect();

        printer.out(FetchedMessages { messages })
    }
}

/// The `imap fetch` output, one block per message.
#[derive(Clone, Debug, Default, Serialize, JsonSchema)]
pub struct FetchedMessages {
    /// The messages, in the order the server returned them.
    pub messages: Vec<FetchedMessage>,
}

/// The data items fetched for one message, each `None` when it was not
/// asked for.
#[derive(Clone, Debug, Default, Serialize, JsonSchema)]
pub struct FetchedMessage {
    /// The message number in the selected mailbox.
    pub seq: u32,
    /// The UID.
    pub uid: Option<u32>,
    /// The flags set on the message.
    pub flags: Option<Vec<String>>,
    /// The internal date, the received-at, as RFC 3339.
    pub internal_date: Option<String>,
    /// The size, in octets.
    pub size: Option<u32>,
    /// The envelope: date, subject and addresses.
    pub envelope: Option<EnvelopeView>,
    /// The MIME body structure tree.
    pub structure: Option<BodyPart>,
}

impl FetchedMessage {
    /// Folds the data items the server returned into one message.
    fn from_items<'a>(seq: u32, items: impl Iterator<Item = MessageDataItem<'a>>) -> Self {
        let mut message = FetchedMessage {
            seq,
            ..Default::default()
        };

        for item in items {
            match item {
                MessageDataItem::Uid(uid) => message.uid = Some(uid.get()),
                MessageDataItem::Envelope(env) => message.envelope = Some(EnvelopeView::from(&env)),
                MessageDataItem::BodyStructure(bs) => message.structure = Some(build_part(&bs)),
                MessageDataItem::Flags(flags) => {
                    message.flags = Some(flags.iter().map(format_flag).collect())
                }
                MessageDataItem::InternalDate(date) => {
                    message.internal_date = Some(date.as_ref().to_rfc3339())
                }
                MessageDataItem::Rfc822Size(size) => message.size = Some(size),
                _ => {}
            }
        }

        message
    }
}

/// A fetched envelope, its addresses already formatted for display.
#[derive(Clone, Debug, Default, Serialize, JsonSchema)]
pub struct EnvelopeView {
    /// The `Date:` header.
    pub date: Option<String>,
    /// The `Subject:` header, RFC 2047 decoded.
    pub subject: Option<String>,
    /// The `Message-ID:` header.
    pub message_id: Option<String>,
    /// The `In-Reply-To:` header.
    pub in_reply_to: Option<String>,
    /// The `From:` addresses.
    pub from: Vec<String>,
    /// The `Sender:` addresses.
    pub sender: Vec<String>,
    /// The `Reply-To:` addresses.
    pub reply_to: Vec<String>,
    /// The `To:` addresses.
    pub to: Vec<String>,
    /// The `Cc:` addresses.
    pub cc: Vec<String>,
    /// The `Bcc:` addresses.
    pub bcc: Vec<String>,
}

impl From<&Envelope<'_>> for EnvelopeView {
    fn from(env: &Envelope<'_>) -> Self {
        Self {
            date: nstring(&env.date),
            subject: nstring(&env.subject).map(|s| decode_mime(&s)),
            message_id: nstring(&env.message_id),
            in_reply_to: nstring(&env.in_reply_to),
            from: env.from.iter().map(format_address).collect(),
            sender: env.sender.iter().map(format_address).collect(),
            reply_to: env.reply_to.iter().map(format_address).collect(),
            to: env.to.iter().map(format_address).collect(),
            cc: env.cc.iter().map(format_address).collect(),
            bcc: env.bcc.iter().map(format_address).collect(),
        }
    }
}

/// One node of a fetched MIME body structure tree.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct BodyPart {
    /// The part's `Content-Type`.
    pub content_type: String,
    /// Its filename, when it names one.
    pub name: Option<String>,
    /// Its size, in octets.
    pub size: Option<usize>,
    /// Its children, empty for a leaf.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<BodyPart>,
}

/// Maps a body structure node onto a display part, recursing through
/// multipart children and encapsulated messages.
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

/// Renders a part's `Content-Type` as `type/subtype`.
fn content_type(specific: &SpecificFields<'_>) -> String {
    match specific {
        SpecificFields::Basic { r#type, subtype } => {
            format!("{}/{}", istring(r#type), istring(subtype))
        }
        SpecificFields::Message { .. } => "message/rfc822".to_string(),
        SpecificFields::Text { subtype, .. } => format!("text/{}", istring(subtype)),
    }
}

/// Reads a part's filename off its `Content-Type` parameters.
fn part_name(basic: &BasicFields<'_>) -> Option<String> {
    basic
        .parameter_list
        .iter()
        .find(|(key, _)| istring(key).eq_ignore_ascii_case("name"))
        .map(|(_, value)| istring(value))
}

/// Renders a fetched flag as its wire spelling.
fn format_flag(flag: &FlagFetch<'_>) -> String {
    match flag {
        FlagFetch::Flag(flag) => flag.to_string(),
        FlagFetch::Recent => "\\Recent".to_string(),
    }
}

/// Renders an IMAP string as UTF-8, lossily.
fn istring(string: &IString<'_>) -> String {
    String::from_utf8_lossy(string.as_ref()).into_owned()
}

/// Renders a nullable IMAP string, `NIL` coming through as `None`.
fn nstring(string: &NString<'_>) -> Option<String> {
    string
        .0
        .as_ref()
        .map(|inner| String::from_utf8_lossy(inner.as_ref()).into_owned())
}

/// Renders a size in human-readable binary units.
fn format_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

impl fmt::Display for FetchedMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;

        for message in &self.messages {
            let id = message.uid.unwrap_or(message.seq);
            writeln!(f, "Message {id}:")?;

            if let Some(flags) = &message.flags {
                let flags = if flags.is_empty() {
                    "(none)".to_string()
                } else {
                    flags.join(" ")
                };
                writeln!(f, "  Flags: {flags}")?;
            }
            if let Some(date) = &message.internal_date {
                writeln!(f, "  Internal date: {date}")?;
            }
            if let Some(size) = message.size {
                writeln!(f, "  Size: {}", format_size(size as usize))?;
            }
            if let Some(envelope) = &message.envelope {
                write_envelope(f, envelope)?;
            }
            if let Some(structure) = &message.structure {
                writeln!(f, "  Structure:")?;
                write_body_tree(f, structure, "    ", true)?;
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

/// Writes an envelope as one indented line per header it carries.
fn write_envelope(f: &mut fmt::Formatter<'_>, env: &EnvelopeView) -> fmt::Result {
    if let Some(date) = &env.date {
        writeln!(f, "  Date: {date}")?;
    }
    if let Some(subject) = &env.subject {
        writeln!(f, "  Subject: {subject}")?;
    }
    if !env.from.is_empty() {
        writeln!(f, "  From: {}", env.from.join(", "))?;
    }
    if !env.sender.is_empty() {
        writeln!(f, "  Sender: {}", env.sender.join(", "))?;
    }
    if !env.reply_to.is_empty() {
        writeln!(f, "  Reply-To: {}", env.reply_to.join(", "))?;
    }
    if !env.to.is_empty() {
        writeln!(f, "  To: {}", env.to.join(", "))?;
    }
    if !env.cc.is_empty() {
        writeln!(f, "  Cc: {}", env.cc.join(", "))?;
    }
    if !env.bcc.is_empty() {
        writeln!(f, "  Bcc: {}", env.bcc.join(", "))?;
    }
    if let Some(message_id) = &env.message_id {
        writeln!(f, "  Message-ID: {message_id}")?;
    }
    if let Some(in_reply_to) = &env.in_reply_to {
        writeln!(f, "  In-Reply-To: {in_reply_to}")?;
    }

    Ok(())
}

/// Writes a body structure as an indented tree.
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
