//! # Envelope
//!
//! An envelope shared across all protocols, plus the normalisation that
//! makes a `Message-ID:` comparable whichever backend surfaced it.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::unified::email::{address::Address, flag::Flag};

/// A message summary, enough to list without fetching a body.
///
/// The mbox cache stores it too, so the kebab-case keys it was written with
/// before 3.0 stay readable through `alias`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// The identifier the backend knows the message by: an IMAP UID, a
    /// JMAP email id, a Maildir filename id.
    pub id: String,
    /// The RFC 5322 section 3.6.4 `Message-ID:`, normalised so it is
    /// stable across every backend storing the message.
    #[serde(default, alias = "message-id")]
    pub message_id: Option<String>,
    /// The messages this one replies to, from its `In-Reply-To:` header.
    ///
    /// A list, the grammar being `1*msg-id`. Each id is normalised like
    /// `messageId`, so a client pairs a reply with its parent
    /// from a listing alone.
    #[serde(default, alias = "in-reply-to")]
    pub in_reply_to: Vec<String>,
    /// The flags set on the message, a sorted set since wire order means
    /// nothing and a duplicate is nonsense.
    #[serde(default)]
    pub flags: BTreeSet<Flag>,
    /// The `Subject:` header, decoded; `null` when the message has none.
    #[serde(default)]
    pub subject: Option<String>,
    /// The senders.
    #[serde(default)]
    pub from: Vec<Address>,
    /// The primary recipients.
    #[serde(default)]
    pub to: Vec<Address>,
    /// The author-claimed send time, from the `Date:` header, `null` when
    /// it is missing or unparseable.
    #[serde(default)]
    pub date: Option<DateTime<FixedOffset>>,
    /// The size of the raw RFC 5322 message, in bytes, `null` when the
    /// backend gives none.
    #[serde(default)]
    pub size: Option<u64>,
    /// Whether the message carries an attachment, `null` when the caller
    /// did not ask or the backend cannot tell.
    #[serde(default, alias = "has-attachment")]
    pub has_attachment: Option<bool>,
}

/// One page of a mailbox's envelopes, and whether a page follows it.
#[derive(Clone, Debug, Default)]
pub struct EnvelopePage {
    /// The envelopes of the page, in the listing's order.
    pub envelopes: Vec<Envelope>,
    /// Whether a page follows this one, never for an unpaged listing.
    pub more: bool,
}

/// Whether a page follows page `page` of `page_size` items in a listing
/// holding `count`, never for an unpaged listing.
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "maildir",
    feature = "mbox",
    feature = "pimdir"
))]
pub fn page_follows(count: u64, page: Option<u32>, page_size: Option<u32>) -> bool {
    let Some(size) = page_size.filter(|size| *size > 0) else {
        return false;
    };

    let page = page.unwrap_or(1).max(1);
    u64::from(page) * u64::from(size) < count
}

/// Splits a raw `In-Reply-To:` value into its bare message ids.
///
/// A backend hands the whole `1*msg-id` value over as one string, so the
/// ids are read off the angle brackets. A value carrying none is split on
/// whitespace, a bare id being commoner than nothing at all.
#[cfg(any(feature = "imap", feature = "gmail"))]
pub fn parse_message_ids(raw: &str) -> Vec<String> {
    if raw.contains('<') {
        return raw
            .split('<')
            .filter_map(|rest| rest.split_once('>'))
            .filter_map(|(id, _)| normalize_message_id(id))
            .collect();
    }

    raw.split_whitespace()
        .filter_map(normalize_message_id)
        .collect()
}

/// Parses an RFC 2822 date, retrying without its day-of-week token.
#[cfg(any(feature = "imap", feature = "gmail"))]
pub fn parse_rfc2822_date(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // NOTE: chrono rejects the whole timestamp when the day-of-week
    // disagrees with the date, which a surprising number of senders get
    // wrong. The token is redundant, so the retry drops it.
    DateTime::parse_from_rfc2822(trimmed)
        .or_else(|_| DateTime::parse_from_rfc2822(strip_weekday(trimmed)))
        .ok()
}

/// Drops the leading day-of-week token of an RFC 2822 date, leaving the
/// unambiguous remainder chrono parses without the weekday check.
#[cfg(any(feature = "imap", feature = "gmail"))]
fn strip_weekday(date: &str) -> &str {
    match date.split_once(", ") {
        Some((dow, rest)) if dow.len() == 3 && dow.bytes().all(|b| b.is_ascii_alphabetic()) => rest,
        _ => date,
    }
}

/// Strips the whitespace and the one pair of angle brackets wrapping a
/// raw `Message-ID:`, an empty result becoming `None`.
///
/// This is what makes [`Envelope::message_id`] comparable byte for byte
/// whichever backend surfaced it.
pub fn normalize_message_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(trimmed)
        .trim();

    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

/// The envelope of a raw message, read from its header block, its id and
/// flags left empty for the backend to fill; `None` when the bytes hold no
/// header field named as RFC 5322 names one, which no message lacks.
///
/// Refs: <https://datatracker.ietf.org/doc/html/rfc5322#section-2.2>
///
/// `with_attachment` also splits the body, to say whether it holds an
/// attachment ([`crate::unified::message::part::has_attachment`]); left
/// `None` otherwise. The subject is read as the read view reads it, raw
/// 8-bit bytes included.
pub fn from_raw(raw: &[u8], with_attachment: bool) -> Option<Envelope> {
    use chrono::TimeZone;
    use mime::{
        header::marker::{DATE, FROM, IN_REPLY_TO, MESSAGE_ID, TO},
        tree::cst::MimeCst,
    };

    use crate::unified::{
        email::address,
        message::{header, part},
    };

    let message = match with_attachment {
        true => MimeCst::parse(raw),
        false => header::parse_headers(raw),
    };
    let root = &message.root;

    // NOTE: mime-rs takes any bytes before a colon for a field name.
    let ftext = |name: &[u8]| {
        let printable = |byte: &u8| matches!(byte, 33..=57 | 59..=126);
        !name.is_empty() && name.iter().all(printable)
    };

    if !root.header.fields().any(|field| ftext(field.name())) {
        return None;
    }

    let addresses = |list: Option<mime::value::address::MimeAddressList>| {
        list.map(|list| address::from_mime(&list))
            .unwrap_or_default()
    };

    let date = root
        .header::<DATE>()
        .filter(|date| date.is_valid())
        .and_then(|date| {
            let offset = FixedOffset::east_opt(i32::from(date.offset) * 60)?;
            offset.timestamp_opt(date.to_unix_seconds(), 0).single()
        });

    let ids = |ids: Option<Vec<String>>| -> Vec<String> {
        ids.unwrap_or_default()
            .iter()
            .filter_map(|id| normalize_message_id(id))
            .collect()
    };

    Some(Envelope {
        id: String::new(),
        message_id: ids(root.header::<MESSAGE_ID>()).into_iter().next(),
        in_reply_to: ids(root.header::<IN_REPLY_TO>()),
        flags: Default::default(),
        subject: header::unstructured(root, "Subject"),
        from: addresses(root.header::<FROM>()),
        to: addresses(root.header::<TO>()),
        date,
        size: Some(raw.len() as u64),
        has_attachment: with_attachment.then(|| part::has_attachment(&message)),
    })
}

#[cfg(all(test, any(feature = "imap", feature = "gmail")))]
mod tests {
    use super::*;

    #[test]
    fn a_reply_to_several_parents_keeps_every_id() {
        assert_eq!(parse_message_ids("<a@x.org>"), ["a@x.org"]);
        assert_eq!(
            parse_message_ids("<a@x.org> <b@x.org>"),
            ["a@x.org", "b@x.org"]
        );

        // NOTE: folded and unspaced values are the same list, the
        // brackets delimiting the ids rather than the whitespace.
        assert_eq!(
            parse_message_ids("<a@x.org>\r\n\t<b@x.org>"),
            ["a@x.org", "b@x.org"]
        );
        assert_eq!(
            parse_message_ids("<a@x.org><b@x.org>"),
            ["a@x.org", "b@x.org"]
        );
    }

    #[test]
    fn a_bracketless_value_still_yields_its_ids() {
        assert_eq!(parse_message_ids("a@x.org"), ["a@x.org"]);
        assert_eq!(parse_message_ids("a@x.org b@x.org"), ["a@x.org", "b@x.org"]);
    }

    #[test]
    fn an_empty_or_unterminated_value_yields_nothing() {
        assert!(parse_message_ids("").is_empty());
        assert!(parse_message_ids("   ").is_empty());
        assert!(parse_message_ids("<>").is_empty());

        assert!(parse_message_ids("<a@x.org").is_empty());
    }

    #[test]
    fn parses_a_well_formed_date() {
        let date = parse_rfc2822_date("Fri, 17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn parses_despite_a_wrong_weekday() {
        let date = parse_rfc2822_date("Thu, 17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn parses_without_a_weekday() {
        let date = parse_rfc2822_date("17 Jul 2026 10:00:00 +0000").unwrap();
        assert_eq!(date.to_rfc3339(), "2026-07-17T10:00:00+00:00");
    }

    #[test]
    fn rejects_empty_and_garbage() {
        assert!(parse_rfc2822_date("   ").is_none());
        assert!(parse_rfc2822_date("not a date").is_none());
    }

    #[test]
    fn an_id_normalises_the_same_way_wherever_it_came_from() {
        // NOTE: the pairing a reply leans on, the parent's `message_id`
        // and the child's `in_reply_to` entry comparing equal.
        assert_eq!(
            parse_message_ids(" <a@x.org> "),
            [normalize_message_id("<a@x.org>").unwrap()]
        );
    }
}

#[cfg(test)]
mod subject_tests {
    use super::from_raw;

    /// A message without a `Subject:` prints `subject` as `null`, not as
    /// an empty string.
    #[test]
    fn a_missing_subject_prints_null() {
        let envelope = from_raw(b"From: a@example.org\r\n\r\nhi\r\n", false).unwrap();
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["subject"], serde_json::Value::Null);

        let envelope = from_raw(b"Subject: Hi\r\n\r\nhi\r\n", false).unwrap();
        assert_eq!(envelope.subject.as_deref(), Some("Hi"));
    }
}
