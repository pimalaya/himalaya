//! # Message parts
//!
//! The one walk over a message's MIME parts that `message read`,
//! `attachment list` and `attachment download` share, so a part id
//! names the same part in all of them, plus what each command reads
//! off a part: its type, role, name, `Content-ID` and decoded bytes.
//!
//! A message is parsed once through [`parse`] into a mime-rs tree, which
//! also refuses a message too complex to be read whole.

use std::borrow::Cow;

use anyhow::Result;
use mime::{
    codec::flowed,
    tree::{body::MimeBodyNode, cst::MimeCst, entity::MimeEntity},
};
use pimalaya_cli::error::CodedError;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{error::ErrorCode, shared::message::header};

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
///
/// Any bytes parse: mime-rs recovers from malformed mail rather than
/// failing on it.
pub fn parse(raw: &[u8]) -> Result<MimeCst<'_>> {
    let message = MimeCst::parse(raw);
    measure(&message.root, 1, &mut Tally::default())?;
    Ok(message)
}

/// The running totals [`measure`] holds a message to.
#[derive(Default)]
struct Tally {
    parts: usize,
    header_lines: usize,
}

/// Measures one part sitting at `depth`, then what it holds: the parts
/// of a multipart one level deeper, and an attached message's top-level
/// part one level deeper too, encoded or not.
fn measure(part: &MimeEntity, depth: usize, tally: &mut Tally) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(too_complex(format!(
            "parts nested deeper than {MAX_DEPTH} levels"
        )));
    }

    tally.parts += 1;
    if tally.parts > MAX_PARTS {
        return Err(too_complex(format!("more than {MAX_PARTS} parts")));
    }

    for field in part.header.fields() {
        let lines = field.line().iter().filter(|&&b| b == b'\n').count();
        tally.header_lines += lines.max(1);
    }
    if tally.header_lines > MAX_HEADER_LINES {
        return Err(too_complex(format!(
            "more than {MAX_HEADER_LINES} header lines"
        )));
    }

    match &part.body {
        MimeBodyNode::Multipart(multipart) => {
            for child in &multipart.parts {
                measure(&child.entity, depth + 1, tally)?;
            }
        }
        MimeBodyNode::Message(inner) => measure(inner, depth + 1, tally)?,
        MimeBodyNode::Leaf(_) | MimeBodyNode::Unparsed(_) => {
            if let Some(inner) = part.nested() {
                measure(&inner.root, depth + 1, tally)?;
            }
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

/// One part of a message as the walk meets it.
struct Node<'t, 'a> {
    part: &'t MimeEntity<'a>,
    /// Its IMAP section, `None` for the message's own multipart and past
    /// [`MAX_DEPTH`].
    section: Option<String>,
}

/// Every part of a message, depth first: the root, then the parts of
/// each multipart in order. An attached message is one part, its own
/// parts being its own, not the message's.
fn nodes<'t, 'a>(message: &'t MimeCst<'a>) -> Vec<Node<'t, 'a>> {
    let root = &message.root;
    let mut nodes = Vec::new();

    match is_multipart(root) {
        true => push(root, None, Some(String::new()), 1, &mut nodes),
        false => push(root, Some(String::from("1")), None, 1, &mut nodes),
    }

    nodes
}

/// Pushes `part`, then its parts when it is a multipart, numbered under
/// `prefix` when it has one.
fn push<'t, 'a>(
    part: &'t MimeEntity<'a>,
    section: Option<String>,
    prefix: Option<String>,
    depth: usize,
    nodes: &mut Vec<Node<'t, 'a>>,
) {
    nodes.push(Node { part, section });

    let MimeBodyNode::Multipart(multipart) = &part.body else {
        return;
    };

    let prefix = prefix.filter(|_| depth <= MAX_DEPTH);

    for (position, child) in multipart.parts.iter().enumerate() {
        let section = prefix
            .as_ref()
            .map(|prefix| format!("{prefix}{}", position + 1));
        let next = section.as_ref().map(|section| format!("{section}."));
        push(&child.entity, section, next, depth + 1, nodes);
    }
}

/// Whether a part is a multipart container.
fn is_multipart(part: &MimeEntity) -> bool {
    matches!(part.body, MimeBodyNode::Multipart(_))
}

/// One leaf part of a message, a `multipart/*` container being none.
pub struct Leaf<'t, 'a> {
    /// The id the `message read` and `attachment` commands share: the
    /// 1-based position of the part among all of the message's parts,
    /// depth first, so the ids gap where a container sits.
    pub id: String,
    pub part: &'t MimeEntity<'a>,
    section: Option<String>,
}

/// The leaf parts of a message, depth first.
///
/// A `message/rfc822` part is a leaf: the parts of the attached message
/// are its own, not the message's.
pub fn leaves<'t, 'a>(message: &'t MimeCst<'a>) -> impl Iterator<Item = Leaf<'t, 'a>> {
    nodes(message)
        .into_iter()
        .enumerate()
        .filter(|(_, node)| !is_multipart(node.part))
        .map(|(index, node)| Leaf {
            id: (index + 1).to_string(),
            part: node.part,
            section: node.section,
        })
}

/// The leaf part carrying the given id.
pub fn find<'t, 'a>(message: &'t MimeCst<'a>, id: &str) -> Option<Leaf<'t, 'a>> {
    leaves(message).find(|leaf| leaf.id == id)
}

/// The leaf part at the given IMAP section (RFC 3501 §6.4.5): `1` for
/// the body of a message that is not multipart, else the 1-based
/// position of each part among its siblings, joined with dots down the
/// multipart parts (`2`, `1.2`).
///
/// A part of an attached message has no section here, as it is no leaf
/// of the message.
pub fn find_section<'t, 'a>(message: &'t MimeCst<'a>, section: &str) -> Option<Leaf<'t, 'a>> {
    leaves(message).find(|leaf| leaf.section.as_deref() == Some(section))
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
pub fn role(part: &MimeEntity) -> PartRole {
    let mime = mime(part);
    let disposition = disposition(part);

    if matches!(part.body, MimeBodyNode::Message(_))
        || mime == "message/rfc822"
        || disposition.as_deref() == Some("attachment")
    {
        return PartRole::Attachment;
    }

    if (mime == "text/plain" || mime == "text/html") && filename(part).is_none() {
        return PartRole::Body;
    }

    if mime.starts_with("image/")
        && (disposition.as_deref() == Some("inline") || content_id(part).is_some())
    {
        return PartRole::Inline;
    }

    PartRole::Attachment
}

/// Whether a leaf part marks its message as having an attachment.
///
/// The pimdir mark rule (STORAGE Annex A.1): a disposition other than
/// `inline` marks (RFC 2183 §2.8); with none, a part marks unless text,
/// multipart or named by a `Content-ID`. Protocol parts never mark.
#[cfg(any(
    feature = "imap",
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub fn marks_attachment(
    mime: &str,
    disposition: Option<&[u8]>,
    has_content_id: bool,
    is_protocol_part: bool,
) -> bool {
    if is_protocol_part {
        return false;
    }

    match disposition {
        Some(kind) if !kind.is_empty() => !kind.eq_ignore_ascii_case(b"inline"),
        _ => !mime.starts_with("text/") && !mime.starts_with("multipart/") && !has_content_id,
    }
}

/// Whether the child at 1-based `position` of a multipart of `subtype` is
/// an RFC 1847 protocol part: the signature of a signed message, or any
/// child of an encrypted one.
#[cfg(any(
    feature = "imap",
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub fn is_protocol_part(subtype: &[u8], position: usize) -> bool {
    subtype.eq_ignore_ascii_case(b"encrypted")
        || (subtype.eq_ignore_ascii_case(b"signed") && position > 1)
}

/// A media type read off the wire, lowercased, `text/plain` when either
/// half is no RFC 2045 token, as [`mime()`] reads a parsed part.
#[cfg(feature = "imap")]
pub fn media_type(kind: &[u8], subtype: &[u8]) -> String {
    if !is_token(kind) || !is_token(subtype) {
        return "text/plain".to_string();
    }

    let kind = String::from_utf8_lossy(kind);
    let subtype = String::from_utf8_lossy(subtype);
    format!("{kind}/{subtype}").to_ascii_lowercase()
}

/// Whether a parsed message has an attachment, see [`marks_attachment`].
///
/// The leaves are the ones [`leaves`] walks, a `message/rfc822` part
/// being one, so the parts of an attached message are not read.
#[cfg(any(
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
pub fn has_attachment(message: &MimeCst) -> bool {
    part_has_attachment(&message.root, false)
}

/// Whether a part, or a leaf under it, marks its message.
#[cfg(any(
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox",
    feature = "pimdir",
    feature = "msgraph"
))]
fn part_has_attachment(part: &MimeEntity, is_protocol: bool) -> bool {
    let MimeBodyNode::Multipart(multipart) = &part.body else {
        return marks_attachment(
            &mime(part),
            disposition(part).as_deref().map(str::as_bytes),
            content_id(part).is_some(),
            is_protocol,
        );
    };

    let mime = mime(part);
    let subtype = mime.strip_prefix("multipart/").unwrap_or_default();
    multipart.parts.iter().enumerate().any(|(index, child)| {
        let is_protocol = is_protocol_part(subtype.as_bytes(), index + 1);
        part_has_attachment(&child.entity, is_protocol)
    })
}

/// The part's MIME type, lowercased, defaulting as RFC 2045 does when
/// the part carries no `Content-Type` (`message/rfc822` in a digest),
/// and taken as `text/plain` when it carries an invalid one (RFC 2045
/// §5.2).
pub fn mime(part: &MimeEntity) -> String {
    if invalid_content_type(part) {
        return String::from("text/plain");
    }

    if part.header.get("Content-Type").is_some() {
        let ctype = part.content_type();
        let top = String::from_utf8_lossy(ctype.top);
        let sub = String::from_utf8_lossy(ctype.sub);
        return format!("{top}/{sub}").to_ascii_lowercase();
    }

    match &part.body {
        MimeBodyNode::Message(_) => "message/rfc822",
        MimeBodyNode::Unparsed(unparsed) if unparsed.digest => "message/rfc822",
        _ => "text/plain",
    }
    .to_string()
}

/// Whether the part's `Content-Type` is present but invalid: a type or
/// subtype missing, empty or not an RFC 2045 token. RFC 2045 §5.2 reads
/// it as `text/plain; charset=us-ascii`.
fn invalid_content_type(part: &MimeEntity) -> bool {
    let Some(value) = header::raw_text(part, "Content-Type") else {
        return false;
    };

    // NOTE: read off the raw value, mime-rs stopping at the first byte
    // past a token, so `text/pl ain` would read as `text/pl`.
    let media = value.split(';').next().unwrap_or_default();
    match media.split_once('/') {
        Some((top, sub)) => !is_token(top.trim().as_bytes()) || !is_token(sub.trim().as_bytes()),
        None => true,
    }
}

/// Whether a value is an RFC 2045 token.
fn is_token(value: &[u8]) -> bool {
    const TSPECIALS: &[u8] = b"()<>@,;:\\\"/[]?=";
    !value.is_empty()
        && value
            .iter()
            .all(|b| b.is_ascii_graphic() && !TSPECIALS.contains(b))
}

/// The `Content-Disposition` type, lowercased.
fn disposition(part: &MimeEntity) -> Option<String> {
    part.disposition()
        .map(|disposition| disposition.kind.to_ascii_lowercase())
}

/// The file name the part carries, from `Content-Disposition` then
/// `Content-Type`, RFC 2231 and RFC 2047 decoded, `None` when empty.
///
/// NOTE: a plain parameter is read off the raw header, as
/// [`header::eight_bit`] reads it, so a raw 8-bit name reads right and
/// adjacent encoded words rebuild a character split between them; the
/// RFC 2231 forms are left to mime-rs.
pub fn filename(part: &MimeEntity) -> Option<String> {
    let disposition = || {
        let value = header::raw_text(part, "Content-Disposition")?;
        if let Some(plain) = header::plain_param(&value, "filename") {
            return Some(header::decode_words(&plain));
        }
        part.disposition()?.param("filename").map(str::to_owned)
    };

    let content_type = || {
        let value = header::raw_text(part, "Content-Type")?;
        if let Some(plain) = header::plain_param(&value, "name") {
            return Some(header::decode_words(&plain));
        }
        part.content_type().param("name")
    };

    let name = disposition()
        .or_else(content_type)
        .or_else(|| part.attachment_name())?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// The `Content-ID` without its angle brackets, `None` when empty.
pub fn content_id(part: &MimeEntity) -> Option<String> {
    let id = header::raw_text(part, "Content-ID")?;
    let id = bare_id(&id);
    (!id.is_empty()).then(|| id.to_owned())
}

/// The `charset` parameter of the `Content-Type`, as found; `us-ascii`
/// for an invalid `Content-Type`.
pub fn charset(part: &MimeEntity) -> Option<String> {
    if invalid_content_type(part) {
        return Some(String::from("us-ascii"));
    }
    param(part, "charset")
}

/// The `method` parameter of a `text/calendar` part, uppercased.
pub fn method(part: &MimeEntity) -> Option<String> {
    if mime(part) != "text/calendar" {
        return None;
    }
    param(part, "method").map(|method| method.to_ascii_uppercase())
}

/// A `Content-Type` parameter, trimmed, `None` when absent or empty.
fn param(part: &MimeEntity, name: &str) -> Option<String> {
    part.header.get("Content-Type")?;
    let value = part.content_type().param(name)?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// The part's bytes once its transfer encoding is undone, in its own
/// charset: what a file attached to the message holds, the attached
/// message itself for a `message/rfc822` part.
pub fn bytes<'t>(part: &'t MimeEntity) -> Cow<'t, [u8]> {
    match &part.body {
        MimeBodyNode::Multipart(_) => Cow::Borrowed(&[]),
        MimeBodyNode::Message(inner) => Cow::Owned(inner.to_bytes()),
        MimeBodyNode::Unparsed(unparsed) => Cow::Borrowed(unparsed.bytes.as_ref()),
        MimeBodyNode::Leaf(raw) => part.decoded().unwrap_or(Cow::Borrowed(raw.as_ref())),
    }
}

/// The part decoded to UTF-8 by its charset, lossily, a `format=flowed`
/// text unwrapped (RFC 3676 §4.2).
///
/// A part labelled US-ASCII, or naming no charset, whose bytes are valid
/// UTF-8 is read as UTF-8, which is what such mail holds in practice.
pub fn text(part: &MimeEntity) -> String {
    let ascii = charset(part).is_none_or(|charset| {
        let charset = charset.trim_matches('"').to_ascii_lowercase();
        charset == "us-ascii" || charset == "ascii"
    });

    // NOTE: mime-rs reads US-ASCII as UTF-8, so bytes past ASCII that are
    // no UTF-8 would come back replaced; Windows-1252 is what such mail
    // holds in practice, as for raw 8-bit header bytes.
    if ascii {
        let text = header::eight_bit(&bytes(part));
        return unflow(part, text);
    }

    match part.text() {
        Some(text) => text.into_owned(),
        None => String::from_utf8_lossy(&bytes(part)).into_owned(),
    }
}

/// Unwraps a `text/plain; format=flowed` text, as mime-rs's own text
/// reading does, any other text going through unchanged.
fn unflow(part: &MimeEntity, text: String) -> String {
    let flowed = mime(part) == "text/plain"
        && param(part, "format").is_some_and(|format| format.eq_ignore_ascii_case("flowed"));

    if !flowed {
        return text;
    }

    let delsp = param(part, "delsp").is_some_and(|delsp| delsp.eq_ignore_ascii_case("yes"));
    flowed::decode(&text, delsp)
}

/// The text body of a message, what a reply quotes: its first text body
/// (RFC 8621 §4.1.4), an HTML one turned into text when the message has
/// no plain text; empty without any.
pub fn body_text(message: &MimeCst) -> String {
    let lists = message.root.body_lists();
    let Some(first) = lists.text.first() else {
        return String::new();
    };

    match mime(first).as_str() {
        "text/html" => html_to_text(&text(first)),
        _ => text(first),
    }
}

/// Reads HTML as plain text: markup dropped with the content of `head`,
/// `style` and `script`, a line break for each break, paragraph, row,
/// list item and heading, the common entities decoded and runs of blank
/// lines collapsed.
fn html_to_text(html: &str) -> String {
    const BREAKS: &[&str] = &[
        "br",
        "p",
        "div",
        "tr",
        "li",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "table",
    ];
    const HIDDEN: &[&str] = &["head", "style", "script", "title"];

    let mut out = String::new();
    let mut rest = html;
    let mut hidden: Option<&str> = None;

    while let Some(open) = rest.find('<') {
        if hidden.is_none() {
            out.push_str(&rest[..open]);
        }

        let Some(close) = rest[open..].find('>') else {
            rest = "";
            break;
        };

        let tag = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];

        let closing = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();

        match hidden {
            Some(open) if closing && name == open => hidden = None,
            Some(_) => (),
            None => match HIDDEN.iter().find(|hidden| **hidden == name) {
                Some(found) if !closing => hidden = Some(found),
                _ if BREAKS.contains(&name.as_str()) => out.push('\n'),
                _ => (),
            },
        }
    }

    if hidden.is_none() {
        out.push_str(rest);
    }

    let out = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&");

    let mut text = String::new();
    let mut blank = 0;
    for line in out.lines().map(str::trim_end) {
        blank = if line.trim().is_empty() { blank + 1 } else { 0 };
        if blank < 2 {
            text.push_str(line);
            text.push('\n');
        }
    }

    text.trim().to_owned()
}

/// An id without the angle brackets around it.
pub fn bare_id(id: &str) -> &str {
    let id = id.trim();
    let id = id.strip_prefix('<').unwrap_or(id);
    id.strip_suffix('>').unwrap_or(id).trim()
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::error::code_of;

    use crate::shared::base64;

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

    #[cfg(any(feature = "maildir", feature = "m2dir", feature = "mbox"))]
    #[test]
    fn a_multipart_without_a_boundary_does_not_count() {
        let raw = b"Content-Type: multipart/mixed\r\n\r\nno parts here\r\n";
        let message = MimeCst::parse(raw);
        assert!(!has_attachment(&message));
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
            .map(|leaf| bytes(leaf.part).into_owned())
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
        raw.extend_from_slice(base64::encode(inner).as_bytes());
        raw.extend_from_slice(b"\r\n--b--\r\n");
        let message = parse(&raw).unwrap();

        let bytes: Vec<Vec<u8>> = leaves(&message)
            .map(|leaf| bytes(leaf.part).into_owned())
            .collect();

        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], inner);
        assert_eq!(bytes[1], inner);
    }
    #[test]
    fn an_html_only_body_is_quoted_as_text() {
        let raw = b"Content-Type: text/html; charset=utf-8\r\n\r\n\
            <html><head><style>p { color: red }</style></head>\
            <body><p>Caf\xc3\xa9 &amp; tea</p><p>at <b>noon</b><br>sharp</p></body></html>\r\n";
        let message = parse(raw).unwrap();

        assert_eq!(body_text(&message), "Café & tea\n\nat noon\nsharp");
    }

    #[test]
    fn a_flowed_text_is_unwrapped_whatever_its_charset() {
        let flowed = "Content-Transfer-Encoding: 8bit\r\n\r\nA line that \r\ngoes on.\r\n";
        for charset in ["us-ascii", "utf-8"] {
            let raw =
                format!("Content-Type: text/plain; charset={charset}; format=flowed\r\n{flowed}");
            let message = parse(raw.as_bytes()).unwrap();
            let leaf = leaves(&message).next().unwrap();

            assert_eq!(
                text(leaf.part).trim_end(),
                "A line that goes on.",
                "{charset}"
            );
        }
    }
}

/// The `hasAttachment` rows, read through the envelope reader the
/// Maildir, m2dir and mbox backends share.
#[cfg(all(test, any(feature = "maildir", feature = "m2dir", feature = "mbox")))]
mod attachment_rows {
    use crate::email::envelope;

    const PDF_ATTACHMENT: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: application/pdf; name=\"a.pdf\"\r\nContent-Disposition: attachment; filename=\"a.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const PDF_ALONE: &[u8] = b"Content-Type: application/pdf; name=\"a.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n";
    const RFC822: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: message/rfc822\r\n\r\nFrom: c@example.org\r\nSubject: inner\r\n\r\ninner\r\n--b--\r\n";
    const PDF_INLINE: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: application/pdf; name=\"a.pdf\"\r\nContent-Disposition: inline; filename=\"a.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const PNG_INLINE: &[u8] = b"Content-Type: multipart/related; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html\r\n\r\n<img src=\"cid:i\">\r\n--b\r\nContent-Type: image/png\r\nContent-ID: <i>\r\nContent-Disposition: inline\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const CSV_ATTACHMENT: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: text/csv; name=\"a.csv\"\r\nContent-Disposition: attachment; filename=\"a.csv\"\r\n\r\na,b\r\n--b--\r\n";
    const PLAIN: &[u8] = b"Content-Type: text/plain\r\n\r\nhello\r\n";
    const ALTERNATIVE: &[u8] = b"Content-Type: multipart/alternative; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: text/html\r\n\r\n<p>hello</p>\r\n--b--\r\n";
    const PNG_BARE: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: image/png; name=\"a.png\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const SIGNED_BARE: &[u8] = b"Content-Type: multipart/signed; protocol=\"application/pgp-signature\"; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: application/pgp-signature; name=\"signature.asc\"\r\n\r\nsig\r\n--b--\r\n";
    const SIGNED_DISPOSED: &[u8] = b"Content-Type: multipart/signed; protocol=\"application/pkcs7-signature\"; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: application/pkcs7-signature; name=\"smime.p7s\"\r\nContent-Disposition: attachment; filename=\"smime.p7s\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const SIGNED_WITH_PDF: &[u8] = b"Content-Type: multipart/signed; protocol=\"application/pgp-signature\"; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: multipart/mixed; boundary=\"c\"\r\n\r\n--c\r\nContent-Type: text/plain\r\n\r\nhello\r\n--c\r\nContent-Type: application/pdf; name=\"a.pdf\"\r\nContent-Disposition: attachment; filename=\"a.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--c--\r\n--b\r\nContent-Type: application/pgp-signature; name=\"signature.asc\"\r\n\r\nsig\r\n--b--\r\n";
    const ENCRYPTED: &[u8] = b"Content-Type: multipart/encrypted; protocol=\"application/pgp-encrypted\"; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: application/pgp-encrypted\r\n\r\nVersion: 1\r\n--b\r\nContent-Type: application/octet-stream\r\n\r\ncipher\r\n--b--\r\n";
    const TEXT_UNKNOWN_DISPOSITION: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: text/plain\r\nContent-Disposition: x-foo\r\n\r\nbye\r\n--b--\r\n";
    const INVALID_TYPE: &[u8] = b"Content-Type: multipart/mixed; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain\r\n\r\nhello\r\n--b\r\nContent-Type: application/x foo\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";
    const PNG_WITH_CONTENT_ID: &[u8] = b"Content-Type: multipart/related; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/html\r\n\r\n<img src=\"cid:i\">\r\n--b\r\nContent-Type: image/png\r\nContent-ID: <i>\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--b--\r\n";

    fn row(raw: &[u8]) -> Option<bool> {
        envelope::from_raw(raw, true).has_attachment
    }

    #[test]
    fn a_part_disposed_as_attachment_counts() {
        assert_eq!(row(PDF_ATTACHMENT), Some(true));
    }

    #[test]
    fn a_single_binary_part_without_disposition_counts() {
        assert_eq!(row(PDF_ALONE), Some(true));
    }

    #[test]
    fn an_attached_message_without_disposition_counts() {
        assert_eq!(row(RFC822), Some(true));
    }

    #[test]
    fn a_binary_part_disposed_inline_does_not_count() {
        assert_eq!(row(PDF_INLINE), Some(false));
    }

    #[test]
    fn an_image_disposed_inline_does_not_count() {
        assert_eq!(row(PNG_INLINE), Some(false));
    }

    #[test]
    fn a_text_part_disposed_as_attachment_counts() {
        assert_eq!(row(CSV_ATTACHMENT), Some(true));
    }

    #[test]
    fn a_text_body_does_not_count() {
        assert_eq!(row(PLAIN), Some(false));
    }

    #[test]
    fn the_bodies_of_an_alternative_do_not_count() {
        assert_eq!(row(ALTERNATIVE), Some(false));
    }

    #[test]
    fn an_image_named_by_a_content_id_does_not_count() {
        assert_eq!(row(PNG_WITH_CONTENT_ID), Some(false));
    }

    #[test]
    fn a_bare_image_counts() {
        assert_eq!(row(PNG_BARE), Some(true));
    }

    #[test]
    fn a_bare_signature_does_not_count() {
        assert_eq!(row(SIGNED_BARE), Some(false));
    }

    #[test]
    fn a_signature_disposed_as_attachment_does_not_count() {
        assert_eq!(row(SIGNED_DISPOSED), Some(false));
    }

    #[test]
    fn an_attachment_under_a_signature_counts() {
        assert_eq!(row(SIGNED_WITH_PDF), Some(true));
    }

    #[test]
    fn the_parts_of_an_encrypted_message_do_not_count() {
        assert_eq!(row(ENCRYPTED), Some(false));
    }

    #[test]
    fn an_unknown_disposition_counts() {
        assert_eq!(row(TEXT_UNKNOWN_DISPOSITION), Some(true));
    }

    #[test]
    fn an_invalid_content_type_reads_as_text() {
        assert_eq!(row(INVALID_TYPE), Some(false));
    }
}
