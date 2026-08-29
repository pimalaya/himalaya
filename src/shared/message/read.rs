//! # Message read
//!
//! The `message read` command, rendering a fetched message as a header
//! block and a walk of its MIME parts.

use std::{
    fmt,
    io::{Write, stdout},
};

use anyhow::{Result, bail};
use clap::Parser;
use humansize::{BINARY, format_size};
use mail_parser::{Addr, Address, ContentType, Message, MessageParser, MessagePart, MimeHeaders};
use pimalaya_cli::printer::{Message as PrinterMessage, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{client::EmailClient, mailbox::arg::MailboxArg},
};

/// Read a message.
///
/// A minimal header block comes first, then one summary line per MIME
/// part with its `Content-*` headers and, for a plain-text part, its
/// decoded contents. An HTML part stays a summary unless it is the sole
/// text part, where its markup is printed instead.
///
/// The `[ID]` prefixing a summary is the part's position in the message,
/// the same id `attachment list` reports and `attachment download` takes.
///
/// `--raw` dumps the original RFC 5322 bytes instead and `--json` the
/// whole parsed message, either of which pipes into an HTML renderer or
/// a pretty-printer of your own.
#[derive(Debug, Parser)]
pub struct MessageReadCommand {
    /// Identifier of the message.
    #[arg(value_name = "ID")]
    pub id: String,
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Write the raw RFC 5322 bytes to stdout.
    ///
    /// With the global `--json` flag the bytes come out as a JSON string
    /// instead, so the output stays valid JSON.
    #[arg(long)]
    pub raw: bool,
    /// Mark the message as seen, the read leaving its flags alone
    /// otherwise.
    #[arg(long)]
    pub seen: bool,
}

impl MessageReadCommand {
    /// Fetches the message and prints it, raw or rendered.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let raw = client.get_message(&mailbox, &self.id, self.seen)?;

        if self.raw {
            if printer.is_json() {
                return printer.out(PrinterMessage::new(String::from_utf8_lossy(&raw)));
            }

            let mut out = stdout().lock();
            out.write_all(&raw)?;
            return Ok(());
        }

        let Some(parsed) = MessageParser::new().parse(&raw) else {
            bail!("Failed to parse RFC 5322 message");
        };

        printer.out(MessageView(parsed.into_owned()))
    }
}

/// The `message read` output: a header block, then one summary line per
/// MIME part with the decoded contents of the text ones.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub struct MessageView(#[schemars(with = "serde_json::Value")] Message<'static>);

impl fmt::Display for MessageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = &self.0;

        if let Some(date) = message.date() {
            writeln!(f, "Date: {}", date.to_rfc822())?;
        }
        if let Some(from) = message.from() {
            writeln!(f, "From: {}", format_address(from))?;
        }
        if let Some(to) = message.to() {
            writeln!(f, "To: {}", format_address(to))?;
        }
        if let Some(cc) = message.cc() {
            writeln!(f, "Cc: {}", format_address(cc))?;
        }
        if let Some(subject) = message.subject() {
            writeln!(f, "Subject: {subject}")?;
        }

        // NOTE: HTML markup is verbose and `--raw` covers it, so a part
        // stays a summary unless printing it is the only readable option.
        let html_only = is_html_only(message);

        // NOTE: a leaf's id is its 1-based position in the whole part
        // list, so the numbering gaps where a skipped container sits.
        // That is what makes it the id the `attachment` commands take.
        for (position, part) in message.parts.iter().enumerate() {
            if part.is_multipart() {
                continue;
            }
            let id = position + 1;

            let mime = part_mime(part);
            let size = format_size(part.len() as u64, BINARY);
            writeln!(f)?;
            match part.attachment_name() {
                Some(name) => writeln!(f, "[{id}] {mime} — {name} ({size})")?,
                None => writeln!(f, "[{id}] {mime} ({size})")?,
            }
            render_part_headers(f, part)?;

            let render_body = if part.is_text_html() {
                html_only
            } else {
                part.is_text()
            };
            if render_body
                && let Some(text) = part
                    .text_contents()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
            {
                writeln!(f)?;
                writeln!(f, "{text}")?;
            }
        }

        Ok(())
    }
}

/// Whether the sole text part of the message is a single HTML one, which
/// is what makes printing its markup worth it.
fn is_html_only(message: &Message) -> bool {
    let mut html = 0;
    let mut plain = 0;
    for part in &message.parts {
        if part.is_text_html() {
            html += 1;
        } else if part.is_text() {
            plain += 1;
        }
    }
    html == 1 && plain == 0
}

/// Renders a part's own `Content-*` headers, indented under its summary
/// line, each shown only when it is present.
fn render_part_headers(f: &mut fmt::Formatter<'_>, part: &MessagePart) -> fmt::Result {
    if let Some(ctype) = part.content_type() {
        writeln!(f, "    Content-Type: {}", format_content_type(ctype))?;
    }
    if let Some(encoding) = part.content_transfer_encoding() {
        writeln!(f, "    Content-Transfer-Encoding: {encoding}")?;
    }
    if let Some(disposition) = part.content_disposition() {
        writeln!(
            f,
            "    Content-Disposition: {}",
            format_content_type(disposition)
        )?;
    }
    if let Some(id) = part.content_id() {
        writeln!(f, "    Content-ID: {id}")?;
    }
    if let Some(description) = part.content_description() {
        writeln!(f, "    Content-Description: {description}")?;
    }
    Ok(())
}

/// Formats a `Content-Type` or `Content-Disposition` value, parameters
/// included.
fn format_content_type(ctype: &ContentType) -> String {
    let mut rendered = match ctype.c_subtype.as_deref() {
        Some(subtype) => format!("{}/{subtype}", ctype.c_type),
        None => ctype.c_type.to_string(),
    };

    if let Some(attributes) = ctype.attributes() {
        for attribute in attributes {
            rendered.push_str(&format!("; {}={}", attribute.name, attribute.value));
        }
    }

    rendered
}

/// The part's MIME type, falling back to its decoded kind when it carries
/// no `Content-Type` header.
fn part_mime(part: &MessagePart) -> String {
    if let Some(ctype) = part.content_type() {
        return match ctype.c_subtype.as_deref() {
            Some(subtype) => format!("{}/{subtype}", ctype.c_type),
            None => ctype.c_type.to_string(),
        };
    }

    if part.is_text_html() {
        "text/html".to_string()
    } else if part.is_text() {
        "text/plain".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// Renders an address header as a comma-separated list, flattening any
/// address group.
fn format_address(address: &Address) -> String {
    let addrs: Vec<String> = match address {
        Address::List(list) => list.iter().map(format_addr).collect(),
        Address::Group(groups) => groups
            .iter()
            .flat_map(|group| group.addresses.iter())
            .map(format_addr)
            .collect(),
    };
    addrs.join(", ")
}

/// Formats one address as `Name <addr>`, or bare when it has no name.
fn format_addr(addr: &Addr) -> String {
    let email = addr.address.as_deref().unwrap_or_default();
    match addr.name.as_deref() {
        Some(name) if !name.is_empty() => format!("{name} <{email}>"),
        _ => email.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(raw: &[u8]) -> String {
        let parsed = MessageParser::new().parse(raw).expect("parse");
        MessageView(parsed.into_owned()).to_string()
    }

    #[test]
    fn plain_single_part_shows_minimal_headers_and_body() {
        let raw = b"Date: Thu, 24 Jul 2025 10:00:00 +0000\r\n\
            From: Alice <alice@example.com>\r\n\
            To: Bob <bob@example.com>\r\n\
            Message-ID: <1@example.com>\r\n\
            X-Mailer: something-verbose\r\n\
            Subject: Hello\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            Hi Bob,\r\nhow are you?\r\n";

        let out = render(raw);

        assert!(out.contains("From: Alice <alice@example.com>"));
        assert!(out.contains("To: Bob <bob@example.com>"));
        assert!(out.contains("Subject: Hello"));
        assert!(!out.contains("Message-ID"));
        assert!(!out.contains("X-Mailer"));

        assert!(out.contains("[1] text/plain"));
        assert!(out.contains("Hi Bob,"));
        assert!(out.contains("how are you?"));
    }

    #[test]
    fn multipart_walks_parts_and_bodies_html_and_attachment() {
        let raw = b"From: Alice <alice@example.com>\r\n\
            Subject: Mixed\r\n\
            Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            plain body\r\n\
            --b\r\n\
            Content-Type: text/html\r\n\
            \r\n\
            <p>html body</p>\r\n\
            --b\r\n\
            Content-Type: application/pdf; name=\"doc.pdf\"\r\n\
            Content-Disposition: attachment; filename=\"doc.pdf\"\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            JVBERi0=\r\n\
            --b--\r\n";

        let out = render(raw);

        // NOTE: the skipped multipart container leaves a gap at id 1, the
        // leaves keeping the `attachment` command ids 2, 3 and 4.
        assert!(out.contains("[2] text/plain"));
        assert!(out.contains("[3] text/html"));
        assert!(out.contains("[4] application/pdf — doc.pdf"));

        assert!(out.contains("    Content-Type: application/pdf; name=doc.pdf"));
        assert!(out.contains("    Content-Transfer-Encoding: base64"));
        assert!(out.contains("    Content-Disposition: attachment; filename=doc.pdf"));

        assert!(out.contains("plain body"));
        assert!(!out.contains("<p>html body</p>"));
        assert!(!out.contains("JVBERi0"));
    }

    #[test]
    fn html_only_message_shows_its_markup() {
        let raw = b"From: Alice <alice@example.com>\r\n\
            Subject: Newsletter\r\n\
            Content-Type: text/html\r\n\
            \r\n\
            <p>hello there</p>\r\n";

        let out = render(raw);

        assert!(out.contains("[1] text/html"));
        assert!(out.contains("<p>hello there</p>"));
    }

    #[test]
    fn html_is_summarized_when_a_plain_alternative_exists() {
        let raw = b"From: Alice <alice@example.com>\r\n\
            Subject: Alt\r\n\
            Content-Type: multipart/alternative; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            plain body\r\n\
            --b\r\n\
            Content-Type: text/html\r\n\
            \r\n\
            <p>html body</p>\r\n\
            --b--\r\n";

        let out = render(raw);

        assert!(out.contains("plain body"));
        assert!(!out.contains("<p>html body</p>"));
    }
}
