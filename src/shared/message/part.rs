//! # Message parts
//!
//! The one walk over a message's MIME parts that `message read`,
//! `attachment list` and `attachment download` share, so a part id
//! names the same part in all of them, plus what each command reads
//! off a part: its type, role, name, `Content-ID` and decoded bytes.
//!
//! A message is parsed once through [`parse`], which also refuses a
//! message too complex to be read whole.

use std::borrow::Cow;

use anyhow::{Result, bail};
use mail_parser::{
    Encoding, HeaderValue, Message, MessageParser, MessagePart, MimeHeaders, PartType,
    parsers::MessageStream,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    error::{CodedError, ErrorCode},
    shared::message::header,
};

/// The deepest a part may sit, the top-level part being at depth 1.
pub const MAX_DEPTH: usize = 8;

/// The most parts a message may hold, containers and the parts of
/// attached messages included.
pub const MAX_PARTS: usize = 200;

/// The most header lines a message may hold, folded lines and the
/// headers of every part included.
pub const MAX_HEADER_LINES: usize = 500;

/// Parses a raw message, refusing it whole with `message-too-complex`
/// when it is nested too deep or holds too many parts or header lines.
pub fn parse(raw: &[u8]) -> Result<Message<'_>> {
    let Some(message) = MessageParser::new().parse(raw) else {
        bail!("Failed to parse RFC 5322 message");
    };

    check_bounds(&message)?;

    Ok(message)
}

/// The running totals [`check_bounds`] holds a message to.
#[derive(Default)]
struct Tally {
    parts: usize,
    header_lines: usize,
}

/// Refuses a message past [`MAX_DEPTH`], [`MAX_PARTS`] or
/// [`MAX_HEADER_LINES`].
fn check_bounds(message: &Message) -> Result<()> {
    measure(message, 1, &mut Tally::default())
}

/// Measures one message whose top-level part sits at `depth`, then the
/// messages attached to it, one level deeper than their part.
///
/// NOTE: a part's depth comes from the byte ranges of the containers
/// still open around it rather than from their lists of sub-parts,
/// which a truncated message leaves empty while its parts still parse.
fn measure(message: &Message, depth: usize, tally: &mut Tally) -> Result<()> {
    let mut open: Vec<u32> = Vec::new();

    for part in &message.parts {
        while open.last().is_some_and(|&end| part.offset_header >= end) {
            open.pop();
        }

        let depth = depth + open.len();
        if depth > MAX_DEPTH {
            return Err(too_complex(format!(
                "parts nested deeper than {MAX_DEPTH} levels"
            )));
        }

        tally.parts += 1;
        if tally.parts > MAX_PARTS {
            return Err(too_complex(format!("more than {MAX_PARTS} parts")));
        }

        for header in &part.headers {
            let span = message
                .raw_message
                .get(header.offset_field as usize..header.offset_end as usize)
                .unwrap_or_default();
            let lines = span.iter().filter(|&&b| b == b'\n').count().max(1);
            tally.header_lines += lines;
        }
        if tally.header_lines > MAX_HEADER_LINES {
            return Err(too_complex(format!(
                "more than {MAX_HEADER_LINES} header lines"
            )));
        }

        match &part.body {
            PartType::Multipart(_) => {
                let end = match part.offset_end {
                    0 => u32::MAX,
                    end => end,
                };
                open.push(end);
            }
            PartType::Message(inner) => measure(inner, depth + 1, tally)?,
            _ => (),
        }
    }

    Ok(())
}

fn too_complex(why: String) -> anyhow::Error {
    CodedError::new(
        ErrorCode::MessageTooComplex,
        format!("Message too complex to be read: {why}"),
    )
    .into()
}

/// One leaf part of a message, a `multipart/*` container being none.
pub struct Leaf<'a, 'x> {
    /// The id the `message read` and `attachment` commands share: the
    /// 1-based position of the part in the whole part list, so the ids
    /// gap where a container sits.
    pub id: String,
    /// The index of the part in [`Message::parts`].
    pub index: usize,
    pub part: &'a MessagePart<'x>,
}

/// The leaf parts of a message, depth first.
///
/// A `message/rfc822` part is a leaf: the parts of the attached message
/// are its own, not the message's.
pub fn leaves<'a, 'x>(message: &'a Message<'x>) -> impl Iterator<Item = Leaf<'a, 'x>> {
    message
        .parts
        .iter()
        .enumerate()
        .filter(|(_, part)| !part.is_multipart())
        .map(|(index, part)| Leaf {
            id: (index + 1).to_string(),
            index,
            part,
        })
}

/// The leaf part carrying the given id.
pub fn find<'a, 'x>(message: &'a Message<'x>, id: &str) -> Option<Leaf<'a, 'x>> {
    leaves(message).find(|leaf| leaf.id == id)
}

/// What a part is to a reader.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PartRole {
    /// Text of the message: a `text/plain` or `text/html` part that
    /// names no file.
    Body,
    /// An image the HTML body shows, marked inline or carrying a
    /// `Content-ID`.
    Inline,
    /// Anything else, attached messages included.
    Attachment,
}

/// The role of a leaf part.
///
/// A `message/rfc822` part or one marked `Content-Disposition:
/// attachment` is an attachment; a `text/plain` or `text/html` part
/// naming no file is a body; an `image/*` part marked inline or carrying
/// a `Content-ID` is inline; anything else is an attachment.
pub fn role(message: &Message, part: &MessagePart) -> PartRole {
    let mime = mime(part);
    let disposition = disposition(part);

    if part.is_message() || mime == "message/rfc822" || disposition.as_deref() == Some("attachment")
    {
        return PartRole::Attachment;
    }

    if (mime == "text/plain" || mime == "text/html") && filename(message, part).is_none() {
        return PartRole::Body;
    }

    if mime.starts_with("image/")
        && (disposition.as_deref() == Some("inline") || content_id(part).is_some())
    {
        return PartRole::Inline;
    }

    PartRole::Attachment
}

/// The part's MIME type, lowercased, defaulting as RFC 2045 does when
/// the part carries no `Content-Type`, and taken as `text/plain` when it
/// carries an invalid one (RFC 2045 §5.2).
pub fn mime(part: &MessagePart) -> String {
    if invalid_content_type(part) {
        return "text/plain".to_string();
    }

    if let Some(ctype) = part.content_type() {
        let subtype = ctype.c_subtype.as_deref().unwrap_or_default();
        return format!("{}/{subtype}", ctype.c_type).to_ascii_lowercase();
    }

    match part.body {
        PartType::Message(_) => "message/rfc822",
        PartType::Html(_) => "text/html",
        PartType::Text(_) => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Whether the part's `Content-Type` is present but invalid: a type or
/// subtype missing, empty or not an RFC 2045 token. RFC 2045 §5.2 reads
/// it as `text/plain; charset=us-ascii`.
fn invalid_content_type(part: &MessagePart) -> bool {
    part.content_type()
        .is_some_and(|ctype| match ctype.c_subtype.as_deref() {
            Some(subtype) => !is_token(&ctype.c_type) || !is_token(subtype),
            None => true,
        })
}

/// Whether a value is an RFC 2045 token.
fn is_token(value: &str) -> bool {
    const TSPECIALS: &[u8] = b"()<>@,;:\\\"/[]?=";
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_graphic() && !TSPECIALS.contains(&b))
}

/// The `Content-Disposition` type, lowercased.
fn disposition(part: &MessagePart) -> Option<String> {
    part.content_disposition()
        .map(|disposition| disposition.c_type.to_ascii_lowercase())
}

/// The file name the part carries, from `Content-Disposition` then
/// `Content-Type`, RFC 2231 and RFC 2047 decoded, `None` when empty.
///
/// NOTE: the header is parsed again from its raw bytes, read as
/// [`header::eight_bit`] does, so a raw 8-bit name reads right and
/// adjacent encoded words rebuild a character split between them.
pub fn filename(message: &Message, part: &MessagePart) -> Option<String> {
    let param = |header: &str, param: &str| {
        let value = header::raw_text(message, &part.headers, header)?;

        // NOTE: mail-parser decodes the encoded words of a quoted
        // parameter one by one, so a plain one is read here; the RFC 2231
        // forms are left to mail-parser.
        if let Some(plain) = header::plain_param(&value, param) {
            return Some(header::decode_words(&plain));
        }

        let line = format!("{value}\n");
        match MessageStream::new(line.as_bytes()).parse_content_type() {
            HeaderValue::ContentType(ctype) => ctype.attribute(param).map(str::to_owned),
            _ => None,
        }
    };

    let name = param("Content-Disposition", "filename")
        .or_else(|| param("Content-Type", "name"))
        .or_else(|| part.attachment_name().map(str::to_owned))?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// The `Content-ID` without its angle brackets, `None` when empty.
pub fn content_id(part: &MessagePart) -> Option<String> {
    let id = bare_id(part.content_id()?);
    (!id.is_empty()).then(|| id.to_owned())
}

/// The `charset` parameter of the `Content-Type`, as found; `us-ascii`
/// for an invalid `Content-Type`.
pub fn charset(part: &MessagePart) -> Option<String> {
    if invalid_content_type(part) {
        return Some("us-ascii".to_string());
    }
    let charset = part.content_type()?.attribute("charset")?.trim();
    (!charset.is_empty()).then(|| charset.to_owned())
}

/// The `method` parameter of a `text/calendar` part, uppercased.
pub fn method(part: &MessagePart) -> Option<String> {
    if mime(part) != "text/calendar" {
        return None;
    }
    let method = part.content_type()?.attribute("method")?.trim();
    (!method.is_empty()).then(|| method.to_ascii_uppercase())
}

/// The part's bytes once its transfer encoding is undone, in its own
/// charset: what a file attached to the message holds.
///
/// NOTE: mail-parser converts a text part to UTF-8 and keeps the whole
/// enclosing message as the raw bytes of an attached one, so both are
/// read back from the part's own byte range instead.
pub fn bytes<'a>(message: &'a Message, part: &'a MessagePart) -> Cow<'a, [u8]> {
    let range = || {
        message
            .raw_message
            .get(part.offset_body as usize..part.offset_end as usize)
            .unwrap_or_default()
    };

    match &part.body {
        PartType::Binary(bytes) | PartType::InlineBinary(bytes) => Cow::Borrowed(bytes.as_ref()),
        PartType::Message(inner) => match part.encoding {
            Encoding::None => Cow::Borrowed(range()),
            _ => Cow::Borrowed(inner.raw_message.as_ref()),
        },
        PartType::Text(_) | PartType::Html(_) => {
            let range = range();
            let (end, bytes) = match part.encoding {
                Encoding::None => return Cow::Borrowed(range),
                Encoding::Base64 => MessageStream::new(range).decode_base64_mime(b""),
                Encoding::QuotedPrintable => {
                    MessageStream::new(range).decode_quoted_printable_mime(b"")
                }
            };
            if end == usize::MAX {
                Cow::Borrowed(part.contents())
            } else {
                bytes
            }
        }
        PartType::Multipart(_) => Cow::Borrowed(&[]),
    }
}

/// The part decoded to UTF-8 by its charset, lossily.
///
/// A part labelled US-ASCII, or naming no charset, whose bytes are valid
/// UTF-8 is read as UTF-8, which is what such mail holds in practice.
pub fn text(message: &Message, part: &MessagePart) -> String {
    let ascii = charset(part).is_none_or(|charset| {
        let charset = charset.trim_matches('"').to_ascii_lowercase();
        charset == "us-ascii" || charset == "ascii"
    });

    if ascii && let Ok(text) = String::from_utf8(bytes(message, part).into_owned()) {
        return text;
    }

    match part.text_contents() {
        Some(text) => text.to_owned(),
        None => String::from_utf8_lossy(part.contents()).into_owned(),
    }
}

/// An id without the angle brackets around it.
pub fn bare_id(id: &str) -> &str {
    let id = id.trim();
    let id = id.strip_prefix('<').unwrap_or(id);
    id.strip_suffix('>').unwrap_or(id).trim()
}

#[cfg(test)]
mod tests {
    use crate::error::code_of;

    use super::*;

    /// A message whose top-level part sits `levels` containers deep, a
    /// text part at the bottom.
    fn nested(levels: usize) -> Vec<u8> {
        let mut raw = String::new();
        for level in 1..levels {
            raw.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"b{level}\"\r\n\r\n--b{level}\r\n"
            ));
        }
        raw.push_str("Content-Type: text/plain\r\n\r\ndeep\r\n");
        for level in (1..levels).rev() {
            raw.push_str(&format!("--b{level}--\r\n"));
        }
        raw.into_bytes()
    }

    /// A `multipart/mixed` message holding `count` text parts.
    fn wide(count: usize) -> Vec<u8> {
        let mut raw = String::from("Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n");
        for n in 0..count {
            raw.push_str(&format!(
                "--b\r\nContent-Type: text/plain\r\n\r\npart {n}\r\n"
            ));
        }
        raw.push_str("--b--\r\n");
        raw.into_bytes()
    }

    /// A single-part message carrying `count` header lines.
    fn tall(count: usize) -> Vec<u8> {
        let mut raw = String::new();
        for n in 1..count {
            raw.push_str(&format!("X-Line-{n}: {n}\r\n"));
        }
        raw.push_str("Content-Type: text/plain\r\n\r\nbody\r\n");
        raw.into_bytes()
    }

    fn refused(raw: &[u8]) -> bool {
        match parse(raw) {
            Ok(_) => false,
            Err(err) => {
                assert_eq!(code_of(&err), Some(ErrorCode::MessageTooComplex));
                true
            }
        }
    }

    #[test]
    fn a_message_at_the_bounds_is_read() {
        assert!(!refused(&nested(MAX_DEPTH)));
        // NOTE: the container is a part too.
        assert!(!refused(&wide(MAX_PARTS - 1)));
        assert!(!refused(&tall(MAX_HEADER_LINES)));
    }

    #[test]
    fn a_message_past_the_bounds_is_refused_with_its_code() {
        assert!(refused(&nested(MAX_DEPTH + 1)));
        assert!(refused(&wide(MAX_PARTS)));
        assert!(refused(&tall(MAX_HEADER_LINES + 1)));
    }

    #[test]
    fn folded_header_lines_count() {
        let mut raw = String::from("Subject: start\r\n");
        for _ in 0..MAX_HEADER_LINES {
            raw.push_str(" more\r\n");
        }
        raw.push_str("Content-Type: text/plain\r\n\r\nbody\r\n");
        assert!(refused(raw.as_bytes()));
    }

    #[test]
    fn sibling_containers_do_not_add_up_in_depth() {
        let mut raw = String::from("Content-Type: multipart/mixed; boundary=\"top\"\r\n\r\n");
        for n in 0..MAX_DEPTH * 2 {
            raw.push_str(&format!(
                "--top\r\nContent-Type: multipart/alternative; boundary=\"s{n}\"\r\n\r\n\
                 --s{n}\r\nContent-Type: text/plain\r\n\r\ntext {n}\r\n--s{n}--\r\n"
            ));
        }
        raw.push_str("--top--\r\n");
        assert!(!refused(raw.as_bytes()));
    }

    #[test]
    fn an_attached_message_nests_one_level_deeper() {
        let mut raw = String::new();
        for level in 1..MAX_DEPTH {
            raw.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"b{level}\"\r\n\r\n--b{level}\r\n"
            ));
        }
        raw.push_str("Content-Type: message/rfc822\r\n\r\nSubject: inner\r\n\r\ninner\r\n");
        for level in (1..MAX_DEPTH).rev() {
            raw.push_str(&format!("--b{level}--\r\n"));
        }
        assert!(refused(raw.as_bytes()));
    }

    #[test]
    fn bytes_undo_the_transfer_encoding_and_keep_the_charset() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: text/plain; charset=ISO-8859-1\r\n\
            Content-Disposition: attachment; filename=\"notes.txt\"\r\n\
            Content-Transfer-Encoding: quoted-printable\r\n\
            \r\n\
            caf=E9=\r\n au lait\r\n\
            --b\r\n\
            Content-Type: text/plain; charset=ISO-8859-1\r\n\
            Content-Disposition: attachment; filename=\"b64.txt\"\r\n\
            Content-Transfer-Encoding: base64\r\n\
            \r\n\
            Y2Fm6Q==\r\n\
            --b\r\n\
            Content-Type: text/plain\r\n\
            \r\n\
            as is\r\n\
            --b--\r\n";
        let message = parse(raw).unwrap();
        let bytes: Vec<Vec<u8>> = leaves(&message)
            .map(|leaf| bytes(&message, leaf.part).into_owned())
            .collect();

        assert_eq!(bytes[0], b"caf\xe9 au lait");
        assert_eq!(bytes[1], b"caf\xe9");
        assert_eq!(bytes[2], b"as is");
    }

    #[test]
    fn bytes_of_an_attached_message_are_that_message() {
        let inner = b"Subject: inner\r\nContent-Type: text/plain\r\n\r\ninner body";
        let mut raw = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\
            \r\n\
            --b\r\n\
            Content-Type: message/rfc822\r\n\
            \r\n"
            .to_vec();
        raw.extend_from_slice(inner);
        raw.extend_from_slice(
            b"\r\n--b\r\nContent-Type: message/rfc822\r\nContent-Transfer-Encoding: base64\r\n\r\n",
        );
        raw.extend_from_slice(
            base64::Engine::encode(&base64::prelude::BASE64_STANDARD, inner).as_bytes(),
        );
        raw.extend_from_slice(b"\r\n--b--\r\n");
        let message = parse(&raw).unwrap();

        let bytes: Vec<Vec<u8>> = leaves(&message)
            .map(|leaf| bytes(&message, leaf.part).into_owned())
            .collect();

        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], inner);
        assert_eq!(bytes[1], inner);
    }
}
