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

/// Read a message from the active account (built-in flag reader).
///
/// Renders a minimal header block (Date, From, To, Cc, Subject) then
/// walks the MIME parts, printing a one-line summary per part with its
/// `Content-*` headers and the decoded contents of plain-text parts. An
/// HTML part is shown as a summary only, unless it is the message's sole
/// text part (an HTML-only mail), in which case its markup is printed.
///
/// Each summary is prefixed with the part's `[ID]` — its position in the
/// message. That is the same id `attachments list` reports, so the id
/// shown here is exactly what you pass to `attachments download <ID>`.
///
/// Pass `--raw` to dump the original RFC 5322 bytes to stdout instead, or
/// `--json` to emit the full parsed message as JSON. For HTML rendering
/// or a custom pretty-printer (`mml interpret`, w3m, your own viewer),
/// pipe the `--raw` output into the renderer of your choice.
#[derive(Debug, Parser)]
pub struct MessageReadCommand {
    /// Identifier of the message (IMAP UID, JMAP email id, or Maildir
    /// filename id).
    #[arg(value_name = "ID")]
    pub id: String,
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Write the raw RFC 5322 bytes to stdout. With the global `--json`
    /// flag the bytes are emitted as a JSON `{ "message": "…" }` string
    /// instead, keeping the output valid JSON.
    #[arg(long)]
    pub raw: bool,
    /// Mark the message as seen while reading it. Backends that offer a
    /// side-effecting fetch (IMAP `BODY[]`) do this in a single round;
    /// the others issue a separate flag update.
    #[arg(long)]
    pub seen: bool,
}

impl MessageReadCommand {
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

/// Parsed message rendered for reading: a minimal header block followed
/// by a per-part walk (one summary line each, plus the decoded contents
/// of plain-text parts), or the raw parsed message as JSON.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub struct MessageView(#[schemars(with = "serde_json::Value")] Message<'static>);

impl fmt::Display for MessageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = &self.0;

        // A minimal, common header set, each line shown only when the
        // header is present. The full header list is one `--json` (or
        // `--raw`) away for anyone who needs it.
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

        // An HTML part is normally summarized, not dumped (its markup is
        // verbose and `--raw` covers it). The exception is an HTML-only
        // mail: when the sole text part is a single HTML one, printing its
        // markup beats printing nothing readable.
        let html_only = is_html_only(message);

        // Walk the parts in order, skipping the multipart containers
        // (they carry no content of their own). Each leaf's id is its
        // 1-based position in the part list, so the numbering has gaps
        // where containers sit and matches the ids used by the
        // `attachments` commands (`attachments download <ID>`). Under the
        // summary line come the part's own MIME headers, then the decoded
        // body for text parts we render (all plain-text parts, plus a lone
        // HTML part); other HTML and binary parts stay a summary.
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

/// Whether the message's only text-based part is a single HTML part (an
/// HTML-only mail with no plain-text alternative). Used to decide whether
/// to print an HTML part's markup rather than just summarize it.
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

/// Renders a part's own MIME headers, indented under its summary line:
/// the `Content-*` family (type with parameters, transfer encoding,
/// disposition, id, description), each shown only when present.
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

/// Formats a `Content-Type` / `Content-Disposition` value as
/// `type[/subtype][; name=value ...]`, keeping its parameters (charset,
/// filename, boundary, ...).
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

/// The part's MIME type (e.g. `text/plain`), falling back to the decoded
/// part kind when the part carries no `Content-Type` header.
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

/// Renders an address header (`From`, `To`, `Cc`) as a comma-separated
/// list of `Name <addr>` entries, flattening any address groups.
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

/// Formats a single address as `Name <addr>`, or just the address when
/// it has no display name.
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

        // Minimal header set, and only that set (no Message-ID/X-Mailer).
        assert!(out.contains("From: Alice <alice@example.com>"));
        assert!(out.contains("To: Bob <bob@example.com>"));
        assert!(out.contains("Subject: Hello"));
        assert!(!out.contains("Message-ID"));
        assert!(!out.contains("X-Mailer"));

        // One part summary plus the decoded body.
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

        // The multipart container is parts[0], so it is skipped and its
        // index (1) becomes a gap: the leaves are numbered by their part
        // position (2, 3, 4), aligning with the `attachments` ids.
        assert!(out.contains("[2] text/plain"));
        assert!(out.contains("[3] text/html"));
        assert!(out.contains("[4] application/pdf — doc.pdf"));

        // Each part shows its own MIME headers.
        assert!(out.contains("    Content-Type: application/pdf; name=doc.pdf"));
        assert!(out.contains("    Content-Transfer-Encoding: base64"));
        assert!(out.contains("    Content-Disposition: attachment; filename=doc.pdf"));

        // Plain-text contents are shown; HTML markup and binary are not.
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

        // The sole text part is HTML, so its markup is printed.
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

        // A plain-text alternative is present, so HTML stays a summary.
        assert!(out.contains("plain body"));
        assert!(!out.contains("<p>html body</p>"));
    }
}
