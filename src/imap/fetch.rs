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
use serde::Serialize;

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
    utils::{decode_mime, format_address},
};

/// Fetch IMAP message data items (FETCH, RFC 3501).
///
/// Fetches the selected data items for every message in the sequence
/// set and prints them per message. Choose items with the flags below;
/// with none, `--envelope` is assumed. The UID is always fetched.
#[derive(Debug, Parser)]
pub struct ImapFetchCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g. "1", "1,2,3", "1:*").
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,

    /// Fetch the envelope (date, subject, from, to, cc, ...).
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
    /// Fetch the size in octets.
    #[arg(long)]
    pub size: bool,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapFetchCommand {
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

/// Renderable list of FETCH results, one block per message.
#[derive(Clone, Debug, Default, Serialize)]
pub struct FetchedMessages {
    pub messages: Vec<FetchedMessage>,
}

/// The fetched data items of a single message.
#[derive(Clone, Debug, Default, Serialize)]
pub struct FetchedMessage {
    pub seq: u32,
    pub uid: Option<u32>,
    pub flags: Option<Vec<String>>,
    pub internal_date: Option<String>,
    pub size: Option<u32>,
    pub envelope: Option<EnvelopeView>,
    pub structure: Option<BodyPart>,
}

impl FetchedMessage {
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

/// Display view of a fetched message envelope.
#[derive(Clone, Debug, Default, Serialize)]
pub struct EnvelopeView {
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
#[derive(Clone, Debug, Serialize)]
pub struct BodyPart {
    pub content_type: String,
    pub name: Option<String>,
    pub size: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<BodyPart>,
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

fn format_flag(flag: &FlagFetch<'_>) -> String {
    match flag {
        FlagFetch::Flag(flag) => flag.to_string(),
        FlagFetch::Recent => "\\Recent".to_string(),
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
