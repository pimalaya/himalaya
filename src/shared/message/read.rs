//! # Message read
//!
//! The `message read` command, rendering a fetched message as a header
//! block and a walk of its MIME parts, or under `--json` as a designed
//! view of its headers, bodies and parts.

use std::{
    fmt,
    io::{Write, stdout},
};

use anyhow::Result;
use chrono::{FixedOffset, NaiveDate, TimeZone};
use clap::Parser;
use humansize::{BINARY, format_size};
use mail_parser::{Addr, Address, ContentType, HeaderValue, Message, MessagePart, MimeHeaders};
use pimalaya_cli::{
    printer::{Message as PrinterMessage, Printer},
    table::sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        mailbox::arg::MailboxArg,
        message::{
            header,
            part::{self, PartRole},
        },
    },
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
/// `--json` prints a view of the message, the same whatever the backend:
/// its headers decoded, its text and HTML bodies decoded to UTF-8, and
/// every leaf part with its id, role (body, inline or attachment), type,
/// file name and size. `--raw` dumps the original RFC 5322 bytes instead.
/// Plain output replaces terminal control and bidi characters, retaining
/// body tabs and newlines. JSON and raw output keep their contents.
///
/// A message nested deeper than 8 levels, or holding more than 200 parts
/// or 500 header lines, is refused whole with the `message-too-complex`
/// code rather than read in part.
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
        let mailbox = self.mailbox.resolve(account);
        let raw = client.get_message(&mailbox, &self.id, self.seen)?;

        if self.raw {
            if printer.is_json() {
                return printer.out(PrinterMessage::new(String::from_utf8_lossy(&raw)));
            }

            let mut out = stdout().lock();
            out.write_all(&raw)?;
            return Ok(());
        }

        printer.out(MessageReadOutput::from_raw(&raw)?)
    }
}

/// The `message read` output: under `--json` the view of the message,
/// built from its raw bytes so it is the same on every backend; as text
/// a header block, then one summary line per MIME part with the decoded
/// contents of the text ones.
#[derive(Serialize, JsonSchema)]
pub struct MessageReadOutput {
    /// The headers a reader acts on, decoded.
    pub headers: MessageHeaders,
    /// Every `text/plain` body part decoded to UTF-8, in order, joined by
    /// a blank line; `null` when there is none.
    pub text: Option<String>,
    /// The first `text/html` body part decoded to UTF-8; `null` when
    /// there is none.
    pub html: Option<String>,
    /// Every leaf part, depth first.
    pub parts: Vec<MessagePartView>,
    #[serde(skip)]
    view: MessageView,
}

impl MessageReadOutput {
    /// Parses a raw message within the bounds of [`part::parse`] and
    /// builds its view: what `message read` and `message parse` print.
    pub fn from_raw(raw: &[u8]) -> Result<Self> {
        Ok(Self::new(part::parse(raw)?))
    }

    /// Builds the view of a parsed message.
    pub fn new(message: Message<'_>) -> Self {
        let mut texts = Vec::new();
        let mut html = None;
        let mut parts = Vec::new();

        for leaf in part::leaves(&message) {
            let role = part::role(&message, leaf.part);
            let mime = part::mime(leaf.part);

            if role == PartRole::Body {
                if mime == "text/plain" {
                    texts.push(part::text(&message, leaf.part));
                } else if mime == "text/html" && html.is_none() {
                    html = Some(part::text(&message, leaf.part));
                }
            }

            parts.push(MessagePartView {
                id: leaf.id,
                role,
                filename: part::filename(&message, leaf.part),
                size: part::bytes(&message, leaf.part).len() as u64,
                content_id: part::content_id(leaf.part),
                charset: part::charset(leaf.part),
                method: part::method(leaf.part),
                mime,
            });
        }

        Self {
            headers: MessageHeaders::new(&message),
            text: (!texts.is_empty()).then(|| texts.join("\n\n")),
            html,
            parts,
            view: MessageView(message.into_owned()),
        }
    }
}

impl fmt::Display for MessageReadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.view.fmt(f)
    }
}

/// The headers of a read message.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeaders {
    pub from: Vec<MailAddress>,
    pub to: Vec<MailAddress>,
    pub cc: Vec<MailAddress>,
    pub bcc: Vec<MailAddress>,
    pub reply_to: Vec<MailAddress>,
    pub sender: Vec<MailAddress>,
    /// The subject, RFC 2047 decoded.
    pub subject: Option<String>,
    /// The date as RFC 3339, with the offset the header carries; `null`
    /// when absent or unreadable.
    pub date: Option<String>,
    /// The `Message-ID`, without angle brackets.
    pub message_id: Option<String>,
    /// The first id of `In-Reply-To`, without angle brackets.
    pub in_reply_to: Option<String>,
    /// The ids of `References`, in order, without angle brackets.
    pub references: Vec<String>,
    /// The `List-ID` header as found, decoded.
    pub list_id: Option<String>,
    /// The `List-Unsubscribe` header as found, decoded.
    pub list_unsubscribe: Option<String>,
    /// The `Precedence` header as found, decoded.
    pub precedence: Option<String>,
    /// The `Auto-Submitted` header as found, decoded.
    pub auto_submitted: Option<String>,
    /// The entries of each address header that are not readable as an
    /// address, dropped from the lists above.
    pub invalid: InvalidAddresses,
}

/// The entries of the address headers that are not readable as an
/// address, as decoded text, in header order. A group name is not an
/// entry, nor is an empty group.
#[derive(Debug, Default, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvalidAddresses {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
    pub from: Vec<String>,
    pub sender: Vec<String>,
}

impl MessageHeaders {
    fn new(message: &Message) -> Self {
        let headers = &message.root_part().headers;
        let text = |name| header::unstructured(message, headers, name);

        let mut invalid = InvalidAddresses::default();
        let (from, from_invalid) = addresses(message, "From");
        let (to, to_invalid) = addresses(message, "To");
        let (cc, cc_invalid) = addresses(message, "Cc");
        let (bcc, bcc_invalid) = addresses(message, "Bcc");
        let (reply_to, reply_to_invalid) = addresses(message, "Reply-To");
        let (sender, sender_invalid) = addresses(message, "Sender");
        invalid.from = from_invalid;
        invalid.to = to_invalid;
        invalid.cc = cc_invalid;
        invalid.bcc = bcc_invalid;
        invalid.reply_to = reply_to_invalid;
        invalid.sender = sender_invalid;

        Self {
            from,
            to,
            cc,
            bcc,
            reply_to,
            sender,
            invalid,
            subject: text("Subject"),
            date: message.date().and_then(rfc3339),
            message_id: message.message_id().and_then(id),
            in_reply_to: ids(message.in_reply_to()).into_iter().next(),
            references: ids(message.references()),
            list_id: text("List-ID"),
            list_unsubscribe: text("List-Unsubscribe"),
            precedence: text("Precedence"),
            auto_submitted: text("Auto-Submitted"),
        }
    }
}

/// One address of an address header.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, JsonSchema)]
pub struct MailAddress {
    /// The display name, RFC 2047 decoded; `null` when absent.
    pub name: Option<String>,
    pub email: String,
}

/// One leaf part of a read message.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessagePartView {
    /// The id `attachment list` reports and `attachment download` takes.
    pub id: String,
    pub role: PartRole,
    /// The MIME type, lowercased.
    pub mime: String,
    /// The file name, RFC 2231 and RFC 2047 decoded; `null` when the part
    /// names none.
    pub filename: Option<String>,
    /// The size of the part once its transfer encoding is undone, in
    /// bytes.
    pub size: u64,
    /// The `Content-ID`, without angle brackets.
    pub content_id: Option<String>,
    /// The `charset` parameter, as found.
    pub charset: Option<String>,
    /// The `method` parameter of a `text/calendar` part, uppercased.
    pub method: Option<String>,
}

/// An address header as a list, read from its raw bytes: groups
/// flattened, names unquoted and RFC 2047 decoded, entries without a
/// valid email dropped and returned beside the list as decoded text.
fn addresses(message: &Message, name: &str) -> (Vec<MailAddress>, Vec<String>) {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();

    let Some(value) = header::raw_text(message, &message.root_part().headers, name) else {
        return (valid, invalid);
    };

    for item in split_addresses(&value) {
        // NOTE: what is left of an empty group, or of a list ending in a
        // separator, holds nothing and is no entry.
        if strip_comments(&item).trim().is_empty() {
            continue;
        }

        match mailbox(&item) {
            Some(address) => valid.push(address),
            None => invalid.push(header::decode_words(item.trim()).trim().to_owned()),
        }
    }

    (valid, invalid)
}

/// Splits an address list on the commas and semicolons outside quotes,
/// comments and angle brackets, dropping group names.
fn split_addresses(value: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut escaped, mut comment, mut angle) = (false, false, 0usize, false);

    for c in value.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && (quoted || comment > 0) {
            current.push(c);
            escaped = true;
            continue;
        }
        if quoted {
            quoted = c != '"';
            current.push(c);
            continue;
        }
        if comment > 0 {
            match c {
                '(' => comment += 1,
                ')' => comment -= 1,
                _ => (),
            }
            current.push(c);
            continue;
        }
        match c {
            '"' => quoted = true,
            '(' => comment = 1,
            '<' => angle = true,
            '>' => angle = false,
            ',' | ';' if !angle => {
                items.push(std::mem::take(&mut current));
                continue;
            }
            ':' if !angle => {
                // NOTE: what precedes is a group name.
                current.clear();
                continue;
            }
            _ => (),
        }
        current.push(c);
    }

    items.push(current);
    items
}

/// One address of a list: `email`, `<email>` or `name <email>`.
fn mailbox(item: &str) -> Option<MailAddress> {
    let item = strip_comments(item);
    let item = item.trim();

    let (name, email) = match (item.find('<'), item.rfind('>')) {
        (Some(open), Some(close)) if open < close => (&item[..open], &item[open + 1..close]),
        _ => ("", item),
    };

    let email = email.trim();
    if !is_email(email) {
        return None;
    }

    let name = header::decode_words(&unquote(name));
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");

    Some(MailAddress {
        name: (!name.is_empty()).then_some(name),
        email: email.to_owned(),
    })
}

/// A value without its comments, quoted strings left alone.
fn strip_comments(value: &str) -> String {
    let mut out = String::new();
    let (mut quoted, mut escaped, mut comment) = (false, false, 0usize);

    for c in value.chars() {
        if comment > 0 {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '(') => comment += 1,
                (false, ')') => {
                    comment -= 1;
                    if comment == 0 {
                        out.push(' ');
                    }
                }
                _ => (),
            }
            continue;
        }
        if quoted {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                _ => (),
            }
            out.push(c);
            continue;
        }
        match c {
            '"' => quoted = true,
            '(' => {
                comment = 1;
                continue;
            }
            _ => (),
        }
        out.push(c);
    }

    out
}

/// A display name without its quotes and backslash escapes.
fn unquote(value: &str) -> String {
    let mut out = String::new();
    let (mut quoted, mut escaped) = (false, false);

    for c in value.chars() {
        match (quoted, escaped, c) {
            (true, true, _) => {
                out.push(c);
                escaped = false;
            }
            (true, false, '\\') => escaped = true,
            (_, _, '"') => quoted = !quoted,
            _ => out.push(c),
        }
    }

    out
}

/// Whether an address is a dot-atom `local@domain` (RFC 5322 §3.4.1,
/// UTF-8 allowed as RFC 6532 does) with a domain of two labels or more.
///
/// NOTE: a quoted local part and a domain literal are refused, as no
/// reader relies on them and they hide what an address says.
fn is_email(email: &str) -> bool {
    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };

    let atext =
        |c: char| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~-".contains(c) || is_utf8_text(c);
    let local_ok = !local.is_empty()
        && local.len() <= 64
        && local
            .split('.')
            .all(|atom| !atom.is_empty() && atom.chars().all(atext));

    let label_ok = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || is_utf8_text(c))
    };
    let domain_ok =
        domain.len() <= 255 && domain.split('.').count() >= 2 && domain.split('.').all(label_ok);

    local_ok && domain_ok
}

/// A non-ASCII character that is neither a control nor a space.
fn is_utf8_text(c: char) -> bool {
    !c.is_ascii() && !c.is_control() && !c.is_whitespace()
}

/// A parsed date as RFC 3339 with its own offset, `None` when it names
/// no real instant.
fn rfc3339(date: &mail_parser::DateTime) -> Option<String> {
    let seconds = i32::from(date.tz_hour) * 3600 + i32::from(date.tz_minute) * 60;
    let offset = if date.tz_before_gmt {
        FixedOffset::west_opt(seconds)?
    } else {
        FixedOffset::east_opt(seconds)?
    };
    let naive = NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())?
        .and_hms_opt(date.hour.into(), date.minute.into(), date.second.into())?;
    let date = offset.from_local_datetime(&naive).single()?;
    Some(date.to_rfc3339())
}

/// A message id without angle brackets, `None` when empty.
fn id(value: &str) -> Option<String> {
    let id = part::bare_id(value);
    (!id.is_empty()).then(|| id.to_owned())
}

/// The ids an `In-Reply-To` or `References` header holds.
fn ids(value: &HeaderValue) -> Vec<String> {
    match value {
        HeaderValue::Text(text) => id(text).into_iter().collect(),
        HeaderValue::TextList(list) => list.iter().filter_map(|text| id(text)).collect(),
        _ => Vec::new(),
    }
}

/// The text rendering of a read message.
struct MessageView(Message<'static>);

impl fmt::Display for MessageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = &self.0;

        if let Some(date) = message.date() {
            writeln!(f, "Date: {}", date.to_rfc822())?;
        }
        if let Some(from) = message.from() {
            writeln!(f, "From: {}", sanitize(&format_address(from)))?;
        }
        if let Some(to) = message.to() {
            writeln!(f, "To: {}", sanitize(&format_address(to)))?;
        }
        if let Some(cc) = message.cc() {
            writeln!(f, "Cc: {}", sanitize(&format_address(cc)))?;
        }
        if let Some(subject) = message.subject() {
            writeln!(f, "Subject: {}", sanitize(subject))?;
        }

        // NOTE: HTML markup is verbose and `--raw` covers it, so a part
        // stays a summary unless printing it is the only readable option.
        let html_only = is_html_only(message);

        // NOTE: the ids are the leaves' own, the ones the `attachment`
        // commands and the JSON view take.
        for part::Leaf { id, part, .. } in part::leaves(message) {
            let mime = part_mime(part);
            let mime = sanitize(&mime);
            let size = format_size(part.len() as u64, BINARY);
            writeln!(f)?;
            match part.attachment_name() {
                Some(name) => writeln!(f, "[{id}] {mime} — {} ({size})", sanitize(name))?,
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
                writeln!(f, "{}", sanitize_body(text))?;
            }
        }

        Ok(())
    }
}

/// Replaces unsafe body characters while retaining its text layout.
fn sanitize_body(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .map(|c| {
            if (c.is_control() && !matches!(c, '\t' | '\n'))
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            {
                char::REPLACEMENT_CHARACTER
            } else {
                c
            }
        })
        .collect()
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
        writeln!(
            f,
            "    Content-Type: {}",
            sanitize(&format_content_type(ctype))
        )?;
    }
    if let Some(encoding) = part.content_transfer_encoding() {
        writeln!(f, "    Content-Transfer-Encoding: {}", sanitize(encoding))?;
    }
    if let Some(disposition) = part.content_disposition() {
        writeln!(
            f,
            "    Content-Disposition: {}",
            sanitize(&format_content_type(disposition))
        )?;
    }
    if let Some(id) = part.content_id() {
        writeln!(f, "    Content-ID: {}", sanitize(id))?;
    }
    if let Some(description) = part.content_description() {
        writeln!(f, "    Content-Description: {}", sanitize(description))?;
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
    use base64::{Engine, prelude::BASE64_STANDARD};

    use super::*;

    fn render(raw: &[u8]) -> String {
        MessageReadOutput::new(part::parse(raw).expect("parse")).to_string()
    }

    #[test]
    fn bodies_do_not_emit_terminal_commands_or_bidi_controls() {
        let text =
            "first\tline\r\n日本語\n\x1b[2J\x1b]52;c;YXVkaXQ=\x07\u{9b}\x7f\u{202e}end\rtail";
        let raw = format!(
            "Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
            BASE64_STANDARD.encode(text)
        );
        let view = MessageReadOutput::from_raw(raw.as_bytes()).unwrap();
        let rendered = view.to_string();
        assert!(
            !rendered
                .chars()
                .any(|c| (c.is_control() && !matches!(c, '\t' | '\n'))
                    || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        );
        assert!(rendered.contains("first\tline\n日本語\n"));
        assert!(rendered.contains("�[2J�]52;c;YXVkaXQ=�"));
        assert_eq!(view.text.as_deref(), Some(text));
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["text"], text);
    }

    #[test]
    fn message_headers_and_part_metadata_are_sanitized() {
        let encoded = format!(
            "=?utf-8?B?{}?=",
            BASE64_STANDARD.encode("audit\x1b]52;c;YQ==\x07\u{202e}")
        );
        let raw = format!(
            "From: {encoded} <audit@example.test>\r\nTo: {encoded} <to@example.test>\r\nCc: {encoded} <cc@example.test>\r\nSubject: {encoded}\r\nContent-Type: text/plain; name=\"{encoded}\"\r\nContent-Disposition: attachment; filename=\"{encoded}\"\r\nContent-Description: {encoded}\r\nContent-ID: <audit\x1b[2J@example.test>\r\n\r\nbody"
        );
        let rendered = render(raw.as_bytes());
        assert!(!rendered.contains('\x1b'));
        assert!(!rendered.contains('\x07'));
        assert!(!rendered.contains('\u{202e}'));
        assert!(rendered.contains("Subject: audit�]52;c;YQ==��"));
    }

    #[test]
    fn html_only_body_is_sanitized_too() {
        let raw = b"Content-Type: text/html\r\n\r\n<p>hello\x1b[2J</p>";
        let rendered = render(raw);
        assert!(rendered.contains("<p>hello�[2J</p>"));
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

    fn view(raw: &[u8]) -> serde_json::Value {
        serde_json::to_value(MessageReadOutput::new(part::parse(raw).expect("parse"))).unwrap()
    }

    #[test]
    fn json_view_has_the_designed_shape() {
        let raw = b"Date: Thu, 24 Jul 2025 10:00:00 +0000\r\n\
            From: Alice <alice@example.com>\r\n\
            Subject: Hello\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            Hi Bob\r\n";

        let json = view(raw);

        // NOTE: serde_json sorts the keys of a parsed object.
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["headers", "html", "parts", "text"]);
        let headers: Vec<&str> = json["headers"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            headers,
            [
                "autoSubmitted",
                "bcc",
                "cc",
                "date",
                "from",
                "inReplyTo",
                "invalid",
                "listId",
                "listUnsubscribe",
                "messageId",
                "precedence",
                "references",
                "replyTo",
                "sender",
                "subject",
                "to",
            ]
        );
        assert_eq!(json["headers"]["to"], serde_json::json!([]));
        assert_eq!(json["text"], "Hi Bob\r\n");
        assert_eq!(json["html"], serde_json::Value::Null);
        assert_eq!(
            json["parts"],
            serde_json::json!([{
                "id": "1",
                "role": "body",
                "mime": "text/plain",
                "filename": null,
                "size": 8,
                "contentId": null,
                "charset": null,
                "method": null,
            }])
        );
    }

    #[test]
    fn encoded_words_are_decoded_adjacent_ones_joined() {
        let raw = b"From: =?ISO-8859-1?Q?Andr=E9?= Dupont <andre@example.com>\r\n\
            To: =?UTF-8?B?w4lsb2RpZQ==?= <elodie@example.com>\r\n\
            Subject: =?UTF-8?B?w4l0w6k=?= =?UTF-8?Q?_=C3=A0_Paris?=\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        assert_eq!(json["headers"]["subject"], "\u{c9}t\u{e9} \u{e0} Paris");
        assert_eq!(
            json["headers"]["from"],
            serde_json::json!([{ "name": "Andr\u{e9} Dupont", "email": "andre@example.com" }])
        );
        assert_eq!(
            json["headers"]["to"],
            serde_json::json!([{ "name": "\u{c9}lodie", "email": "elodie@example.com" }])
        );
    }

    #[test]
    fn address_groups_are_flattened_and_invalid_entries_dropped() {
        let raw = b"From: alice@example.com\r\n\
            To: Friends: Bob <bob@example.com>, carol@example.com;, dave@example.org\r\n\
            Cc: Undisclosed recipients:;\r\n\
            Bcc: nobody, Eve <eve@>, \"x\" <@example.com>, Frank <frank@localhost>, <grace@example.net>\r\n\
            Reply-To: \"Support, Team\" <support@example.com>\r\n\
            Sender: Mallory <mallory@example.com>\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        assert_eq!(
            json["headers"]["from"],
            serde_json::json!([{ "name": null, "email": "alice@example.com" }])
        );
        assert_eq!(
            json["headers"]["to"],
            serde_json::json!([
                { "name": "Bob", "email": "bob@example.com" },
                { "name": null, "email": "carol@example.com" },
                { "name": null, "email": "dave@example.org" },
            ])
        );
        assert_eq!(json["headers"]["cc"], serde_json::json!([]));
        assert_eq!(
            json["headers"]["bcc"],
            serde_json::json!([{ "name": null, "email": "grace@example.net" }])
        );
        assert_eq!(
            json["headers"]["replyTo"],
            serde_json::json!([{ "name": "Support, Team", "email": "support@example.com" }])
        );
        assert_eq!(
            json["headers"]["sender"],
            serde_json::json!([{ "name": "Mallory", "email": "mallory@example.com" }])
        );
    }

    #[test]
    fn date_keeps_its_offset_and_an_unreadable_one_is_null() {
        let at = |date: &str| {
            let raw = format!("Date: {date}\r\nContent-Type: text/plain\r\n\r\nbody\r\n");
            view(raw.as_bytes())["headers"]["date"].clone()
        };

        assert_eq!(
            at("Thu, 24 Jul 2025 10:00:00 -0500"),
            "2025-07-24T10:00:00-05:00"
        );
        assert_eq!(
            at("Thu, 24 Jul 2025 10:00:00 +0530"),
            "2025-07-24T10:00:00+05:30"
        );
        assert_eq!(
            at("Thu, 24 Jul 2025 10:00:00 +0000"),
            "2025-07-24T10:00:00+00:00"
        );
        assert_eq!(at("not a date"), serde_json::Value::Null);
        assert_eq!(
            at("Mon, 31 Feb 2025 10:00:00 +0000"),
            serde_json::Value::Null
        );
    }

    #[test]
    fn ids_lose_their_angle_brackets() {
        let raw = b"Message-ID: <3@example.com>\r\n\
            In-Reply-To: <2@example.com> <1@example.com>\r\n\
            References: <1@example.com>\r\n <2@example.com>\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        assert_eq!(json["headers"]["messageId"], "3@example.com");
        assert_eq!(json["headers"]["inReplyTo"], "2@example.com");
        assert_eq!(
            json["headers"]["references"],
            serde_json::json!(["1@example.com", "2@example.com"])
        );
    }

    #[test]
    fn list_headers_are_kept_as_found() {
        let raw = b"List-ID: =?UTF-8?Q?Caf=C3=A9?= news <news.example.com>\r\n\
            List-Unsubscribe: <mailto:leave@example.com>,\r\n <https://example.com/leave>\r\n\
            Precedence: bulk\r\n\
            Auto-Submitted: auto-generated\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        assert_eq!(
            json["headers"]["listId"],
            "Caf\u{e9} news <news.example.com>"
        );
        assert_eq!(
            json["headers"]["listUnsubscribe"],
            "<mailto:leave@example.com>, <https://example.com/leave>"
        );
        assert_eq!(json["headers"]["precedence"], "bulk");
        assert_eq!(json["headers"]["autoSubmitted"], "auto-generated");
        assert_eq!(json["headers"]["subject"], serde_json::Value::Null);
    }

    #[test]
    fn bodies_are_decoded_by_their_charset_and_transfer_encoding() {
        let cp1252 = BASE64_STANDARD.encode(b"5 \x80 l\x92heure");
        let raw = format!(
            "Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/plain; charset=ISO-8859-1\r\n\
            Content-Transfer-Encoding: quoted-printable\r\n\
            \r\n\
            caf=E9 cr=E8me=\r\n br=FBl=E9e\r\n\
            --b\r\n\
            Content-Type: text/plain; charset=windows-1252\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            {cp1252}\r\n\
            --b--\r\n"
        );

        let json = view(raw.as_bytes());

        assert_eq!(
            json["text"],
            "caf\u{e9} cr\u{e8}me br\u{fb}l\u{e9}e\n\n5 \u{20ac} l\u{2019}heure"
        );
        assert_eq!(json["parts"][0]["charset"], "ISO-8859-1");
        assert_eq!(json["parts"][1]["charset"], "windows-1252");
        // NOTE: the size counts the bytes in the part's own charset.
        assert_eq!(json["parts"][0]["size"], 17);
        assert_eq!(json["parts"][1]["size"], 11);
    }

    #[test]
    fn an_unknown_charset_reads_lossily() {
        let raw = b"Content-Type: text/plain; charset=x-unknown\r\n\
            \r\n\
            ok \xff\r\n";

        let json = view(raw);

        assert_eq!(json["text"], "ok \u{fffd}\r\n");
    }

    #[test]
    fn nested_alternative_in_mixed_gives_bodies_and_an_attachment() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"outer\"\r\n\
            \r\n\
            --outer\r\n\
            Content-Type: multipart/alternative; boundary=\"inner\"\r\n\
            \r\n\
            --inner\r\n\
            Content-Type: text/plain; charset=utf-8\r\n\
            \r\n\
            plain body\r\n\
            --inner\r\n\
            Content-Type: text/html; charset=utf-8\r\n\
            \r\n\
            <p>html body</p>\r\n\
            --inner--\r\n\
            \r\n\
            --outer\r\n\
            Content-Type: application/pdf; name=\"doc.pdf\"\r\n\
            Content-Disposition: attachment; filename=\"doc.pdf\"\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            JVBERi0=\r\n\
            --outer--\r\n";

        let json = view(raw);

        assert_eq!(json["text"], "plain body");
        assert_eq!(json["html"], "<p>html body</p>");
        let parts: Vec<(&str, &str, &str)> = json["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| {
                (
                    part["id"].as_str().unwrap(),
                    part["role"].as_str().unwrap(),
                    part["mime"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            parts,
            [
                ("3", "body", "text/plain"),
                ("4", "body", "text/html"),
                ("5", "attachment", "application/pdf"),
            ]
        );
        assert_eq!(json["parts"][2]["filename"], "doc.pdf");
        assert_eq!(json["parts"][2]["size"], 5);
    }

    #[test]
    fn an_attached_message_is_one_attachment_part() {
        let raw = b"Subject: Outer\r\n\
            Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            see attached\r\n\
            --b\r\n\
            Content-Type: message/rfc822\r\n\
            \r\n\
            Subject: Inner\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            inner body\r\n\
            --b--\r\n";

        let json = view(raw);

        assert_eq!(json["headers"]["subject"], "Outer");
        assert_eq!(json["text"], "see attached");
        let parts = json["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1]["id"], "3");
        assert_eq!(parts[1]["role"], "attachment");
        assert_eq!(parts[1]["mime"], "message/rfc822");
        assert_eq!(parts[1]["filename"], serde_json::Value::Null);
        let inner = b"Subject: Inner\r\nContent-Type: text/plain\r\n\r\ninner body";
        assert_eq!(parts[1]["size"], inner.len());
    }

    #[test]
    fn rfc2231_filenames_are_decoded() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: application/pdf\r\n\
            Content-Disposition: attachment;\r\n filename*0*=utf-8''r%C3%A9sum;\r\n filename*1*=%C3%A9.pdf\r\n\
            \r\n\
            x\r\n\
            --b\r\n\
            Content-Type: text/plain\r\n\
            Content-Disposition: attachment; filename*=iso-8859-1'fr'caf%E9.txt\r\n\
            \r\n\
            y\r\n\
            --b\r\n\
            Content-Type: application/octet-stream; name=\"=?UTF-8?B?w6l0w6kucG5n?=\"\r\n\
            \r\n\
            z\r\n\
            --b--\r\n";

        let json = view(raw);

        assert_eq!(json["parts"][0]["filename"], "r\u{e9}sum\u{e9}.pdf");
        assert_eq!(json["parts"][1]["filename"], "caf\u{e9}.txt");
        assert_eq!(json["parts"][1]["role"], "attachment");
        assert_eq!(json["parts"][2]["filename"], "\u{e9}t\u{e9}.png");
        assert_eq!(json["text"], serde_json::Value::Null);
    }

    #[test]
    fn roles_follow_the_rule() {
        let raw = b"Content-Type: multipart/related; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/html\r\n\
            \r\n\
            <img src=\"cid:logo@example.com\">\r\n\
            --b\r\n\
            Content-Type: image/png\r\n\
            Content-ID: <logo@example.com>\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            iVBORw==\r\n\
            --b\r\n\
            Content-Type: image/gif\r\n\
            Content-Disposition: inline\r\n\
            \r\n\
            GIF\r\n\
            --b\r\n\
            Content-Type: image/jpeg\r\n\
            \r\n\
            JPG\r\n\
            --b\r\n\
            Content-Type: text/plain; name=\"notes.txt\"\r\n\
            \r\n\
            notes\r\n\
            --b\r\n\
            Content-Type: text/calendar; charset=UTF-8; method=request\r\n\
            \r\n\
            BEGIN:VCALENDAR\r\n\
            END:VCALENDAR\r\n\
            --b\r\n\
            Content-Type: image/png\r\n\
            Content-ID: <photo@example.com>\r\n\
            Content-Disposition: attachment; filename=\"photo.png\"\r\n\
            \r\n\
            PNG\r\n\
            --b--\r\n";

        let json = view(raw);

        let roles: Vec<&str> = json["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| part["role"].as_str().unwrap())
            .collect();
        assert_eq!(
            roles,
            [
                "body",
                "inline",
                "inline",
                "attachment",
                "attachment",
                "attachment",
                "attachment"
            ]
        );
        assert_eq!(json["parts"][1]["contentId"], "logo@example.com");
        assert_eq!(json["parts"][1]["size"], 4);
        assert_eq!(json["parts"][5]["mime"], "text/calendar");
        assert_eq!(json["parts"][5]["method"], "REQUEST");
        assert_eq!(json["parts"][5]["charset"], "UTF-8");
        assert_eq!(json["parts"][0]["method"], serde_json::Value::Null);
        assert_eq!(json["html"], "<img src=\"cid:logo@example.com\">");
        assert_eq!(json["text"], serde_json::Value::Null);
    }

    #[test]
    fn utf8_bytes_labelled_us_ascii_read_as_utf8() {
        let raw = b"From: a@exemple.test\r\nSubject: ASCII\r\nContent-Type: text/plain; charset=us-ascii\r\n\r\nD\xc3\xa9clar\xc3\xa9 ASCII mais en UTF-8";

        let json = view(raw);

        assert_eq!(json["text"], "D\u{e9}clar\u{e9} ASCII mais en UTF-8");
        assert_eq!(json["parts"][0]["charset"], "us-ascii");
    }

    #[test]
    fn non_utf8_bytes_labelled_us_ascii_keep_their_decoding() {
        let raw = b"Content-Type: text/html; charset=us-ascii\r\n\r\n<p>caf\xe9</p>";

        let json = view(raw);

        assert_eq!(json["html"], "<p>caf\u{e9}</p>");
    }

    #[test]
    fn a_raw_8bit_subject_reads_as_windows_1252_or_utf8() {
        let raw = b"From: a@exemple.test\r\nSubject: Caf\xe9 \x80\r\nContent-Type: text/plain; charset=windows-1252\r\n\r\nPrix : 12 \x80 \x96 \x9c fin";

        let json = view(raw);

        assert_eq!(json["headers"]["subject"], "Caf\u{e9} \u{20ac}");
        assert_eq!(json["text"], "Prix : 12 \u{20ac} \u{2013} \u{153} fin");

        let raw = "Subject: Caf\u{e9} \u{20ac}\r\n\r\nx".as_bytes();
        assert_eq!(view(raw)["headers"]["subject"], "Caf\u{e9} \u{20ac}");
    }

    #[test]
    fn raw_8bit_names_and_filenames_read_as_windows_1252() {
        let raw = b"From: Andr\xe9 <andre@example.com>\r\n\
            To: \"Z\xc3\xa9lie\" <zelie@example.com>\r\n\
            Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: application/pdf\r\n\
            Content-Disposition: attachment; filename=\"r\xe9sum\xe9.pdf\"\r\n\
            \r\n\
            x\r\n\
            --b--\r\n";

        let json = view(raw);

        assert_eq!(json["headers"]["from"][0]["name"], "Andr\u{e9}");
        assert_eq!(json["headers"]["to"][0]["name"], "Z\u{e9}lie");
        assert_eq!(json["parts"][0]["filename"], "r\u{e9}sum\u{e9}.pdf");
    }

    #[test]
    fn a_character_split_across_encoded_words_is_rebuilt() {
        let raw = b"From: a@exemple.test\r\nSubject: =?utf-8?B?Q2Fmw6kg?= =?utf-8?B?ww==?= =?utf-8?B?qXTDqQ==?= fin =?iso-8859-1?Q?=E9t=E9?=\r\n\r\nx";

        let json = view(raw);

        assert_eq!(
            json["headers"]["subject"],
            "Caf\u{e9} \u{e9}t\u{e9} fin \u{e9}t\u{e9}"
        );
    }

    #[test]
    fn split_encoded_words_rebuild_names_and_filenames() {
        let raw =
            b"From: =?utf-8?B?w4lsb2Rp?= =?utf-8?B?ZSBD?= =?utf-8?B?w6k=?= <e@example.com>\r\n\
            Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: application/pdf; name=\"=?utf-8?B?w6k=?= =?utf-8?B?dMOp?=.pdf\"\r\n\
            \r\n\
            x\r\n\
            --b--\r\n";

        let json = view(raw);

        assert_eq!(json["headers"]["from"][0]["name"], "\u{c9}lodie C\u{e9}");
        assert_eq!(json["parts"][0]["filename"], "\u{e9}t\u{e9}.pdf");
    }

    #[test]
    fn an_invalid_content_type_is_plain_us_ascii_text() {
        let raw = b"From: a@exemple.test\nSubject: LF seulement\nContent-Type: text/\nDate: pas une date\n\nCorps en LF";

        let json = view(raw);

        assert_eq!(json["text"], "Corps en LF");
        assert_eq!(json["headers"]["subject"], "LF seulement");
        assert_eq!(json["headers"]["date"], serde_json::Value::Null);
        assert_eq!(json["parts"][0]["role"], "body");
        assert_eq!(json["parts"][0]["mime"], "text/plain");
        assert_eq!(json["parts"][0]["charset"], "us-ascii");

        for ctype in ["textplain", "/plain", "text/pl ain", "text/plain/x"] {
            let raw = format!("Content-Type: {ctype}\r\n\r\nD\u{e9}j\u{e0}");
            let json = view(raw.as_bytes());
            assert_eq!(json["parts"][0]["mime"], "text/plain", "{ctype}");
            assert_eq!(json["text"], "D\u{e9}j\u{e0}", "{ctype}");
        }
    }

    #[test]
    fn unreadable_entries_are_listed_apart() {
        let raw = b"From: alice@example.com\r\n\
            To: Team: Bob <bob@example.com>, carol@example.com;, nobody\r\n\
            Cc: a@localhost, \"john doe\"@example.com, =?UTF-8?Q?Caf=C3=A9?=\r\n\
            Bcc: Undisclosed-recipients:;\r\n\
            Reply-To: Empty:;, (a comment), dave@example.org,\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        assert_eq!(
            json["headers"]["to"],
            serde_json::json!([
                { "name": "Bob", "email": "bob@example.com" },
                { "name": null, "email": "carol@example.com" },
            ])
        );
        assert_eq!(
            json["headers"]["invalid"],
            serde_json::json!({
                "to": ["nobody"],
                "cc": ["a@localhost", "\"john doe\"@example.com", "Caf\u{e9}"],
                "bcc": [],
                "replyTo": [],
                "from": [],
                "sender": [],
            })
        );
        assert_eq!(json["headers"]["cc"], serde_json::json!([]));
        assert_eq!(json["headers"]["bcc"], serde_json::json!([]));
        assert_eq!(
            json["headers"]["replyTo"],
            serde_json::json!([{ "name": null, "email": "dave@example.org" }])
        );
    }

    #[test]
    fn valid_headers_list_no_invalid_entry() {
        let raw = b"From: Alice <alice@example.com>\r\n\
            To: bob@example.com, \"Dupont, Anne\" <anne@example.com>\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            body\r\n";

        let json = view(raw);

        for field in ["to", "cc", "bcc", "replyTo", "from", "sender"] {
            assert_eq!(
                json["headers"]["invalid"][field],
                serde_json::json!([]),
                "{field}"
            );
        }
    }

    #[test]
    fn invalid_entries_are_in_header_order() {
        let raw = b"To: x, alice@example.com, y <y@nowhere>, Zed\r\n\r\nbody\r\n";

        let json = view(raw);

        assert_eq!(
            json["headers"]["invalid"]["to"],
            serde_json::json!(["x", "y <y@nowhere>", "Zed"])
        );
    }
}
