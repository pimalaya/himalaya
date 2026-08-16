//! Envelope shared across all protocols.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::email::{address::Address, flag::Flag};

/// Lightweight summary of a message: enough to display in a list
/// without fetching the full body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Envelope {
    /// Backend-specific identifier of the message.
    ///
    /// IMAP UID, JMAP email ID or Maildir filename id.
    pub id: String,

    /// `Message-ID:` header value (RFC 5322 §3.6.4), `None` when the
    /// header is missing or the backend did not surface it. Stable
    /// across every backend that stores the message.
    #[serde(default)]
    pub message_id: Option<String>,

    /// `In-Reply-To:` header value (RFC 5322 §3.6.4), the message(s)
    /// this one replies to, empty when the header is missing or the
    /// backend did not surface it.
    ///
    /// A list because the grammar is `1*msg-id`: one id is the common
    /// case and a reply to a merged thread is not. Each id is
    /// normalised like [`Envelope::message_id`], so the two compare
    /// byte-for-byte and a client can pair a reply with its parent
    /// from a listing alone.
    #[serde(default)]
    pub in_reply_to: Vec<String>,

    /// Flags set on the message. Stored as a sorted set since wire
    /// order is not meaningful and duplicates are nonsensical.
    #[serde(default)]
    pub flags: BTreeSet<Flag>,

    /// Subject header value.
    #[serde(default)]
    pub subject: String,

    /// Sender(s).
    #[serde(default)]
    pub from: Vec<Address>,

    /// Primary recipient(s).
    #[serde(default)]
    pub to: Vec<Address>,

    /// Author-claimed send time, taken from the `Date:` header (IMAP
    /// `ENVELOPE.date`, JMAP `sentAt`, parsed `Date:` for Maildir).
    /// `None` when the header is missing or unparseable.
    #[serde(default)]
    pub date: Option<DateTime<FixedOffset>>,

    /// Size of the raw RFC 5322 message in bytes.
    #[serde(default)]
    pub size: u64,

    /// Whether the message has at least one attachment, when the caller
    /// opted in. `None` when not requested or when detection is not
    /// implemented for the active backend.
    #[serde(default)]
    pub has_attachment: Option<bool>,
}

/// Splits a raw `In-Reply-To:` value into its bare message ids.
///
/// RFC 5322 §3.6.4 gives the field as `1*msg-id`, and a backend hands
/// the whole value over as one string (the IMAP `ENVELOPE`, a Gmail
/// metadata header), so the ids are read off the angle brackets that
/// delimit them. A value carrying none is split on whitespace instead,
/// since a client that wrote a bare id is commoner than a value that
/// means nothing at all. Each id is normalised like
/// [`normalize_message_id`].
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

/// Strips RFC 5322 `msg-id` wrappers from the raw `Message-ID:` value
/// so every backend's [`Envelope::message_id`] is comparable
/// byte-for-byte. Whitespace and a single pair of angle brackets are
/// removed; an empty result becomes `None`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_to_several_parents_keeps_every_id() {
        assert_eq!(parse_message_ids("<a@x.org>"), ["a@x.org"]);
        assert_eq!(
            parse_message_ids("<a@x.org> <b@x.org>"),
            ["a@x.org", "b@x.org"]
        );

        // Folded and unspaced values are the same list: the brackets
        // delimit the ids, not the whitespace between them.
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

        // An id whose closing bracket never arrives is not an id.
        assert!(parse_message_ids("<a@x.org").is_empty());
    }

    #[test]
    fn an_id_normalises_the_same_way_wherever_it_came_from() {
        // The pairing a reply leans on: the parent's `message_id` and
        // the child's `in_reply_to` entry must compare equal.
        assert_eq!(
            parse_message_ids(" <a@x.org> "),
            [normalize_message_id("<a@x.org>").unwrap()]
        );
    }
}
