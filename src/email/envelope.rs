//! # Envelope
//!
//! An envelope shared across all protocols, plus the normalisation that
//! makes a `Message-ID:` comparable whichever backend surfaced it.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::email::{address::Address, flag::Flag};

/// A message summary, enough to list without fetching a body.
///
/// The mbox cache stores it too, so the kebab-case keys it was written with
/// before 3.0 stay readable through `alias`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// [`Envelope::message_id`], so a client pairs a reply with its parent
    /// from a listing alone.
    #[serde(default, alias = "in-reply-to")]
    pub in_reply_to: Vec<String>,
    /// The flags set on the message, a sorted set since wire order means
    /// nothing and a duplicate is nonsense.
    #[serde(default)]
    pub flags: BTreeSet<Flag>,
    /// The `Subject:` header.
    #[serde(default)]
    pub subject: String,
    /// The senders.
    #[serde(default)]
    pub from: Vec<Address>,
    /// The primary recipients.
    #[serde(default)]
    pub to: Vec<Address>,
    /// The author-claimed send time, from the `Date:` header, `None` when
    /// it is missing or unparseable.
    #[serde(default)]
    pub date: Option<DateTime<FixedOffset>>,
    /// The size of the raw RFC 5322 message, in bytes.
    #[serde(default)]
    pub size: u64,
    /// Whether the message carries an attachment, `None` when the caller
    /// did not ask or the backend cannot tell.
    #[serde(default, alias = "has-attachment")]
    pub has_attachment: Option<bool>,
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

/// Strips the whitespace and the one pair of angle brackets wrapping a
/// raw `Message-ID:`, an empty result becoming `None`.
///
/// This is what makes [`Envelope::message_id`] comparable byte for byte
/// whichever backend surfaced it.
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "maildir",
    feature = "m2dir",
    feature = "mbox"
))]
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
/// flags left empty for the backend to fill.
///
/// `with_attachment` also splits the body, to say whether it holds an
/// attachment (RFC 8621 §4.1.4); left `None` otherwise. The subject is
/// read as the read view reads it, raw 8-bit bytes included.
#[cfg(any(feature = "maildir", feature = "m2dir", feature = "mbox"))]
pub fn from_raw(raw: &[u8], with_attachment: bool) -> Envelope {
    use chrono::TimeZone;
    use mime::{
        header::marker::{DATE, FROM, IN_REPLY_TO, MESSAGE_ID, TO},
        tree::cst::MimeCst,
    };

    use crate::{email::address, shared::message::header};

    let message = match with_attachment {
        true => MimeCst::parse(raw),
        false => header::parse_headers(raw),
    };
    let root = &message.root;

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

    Envelope {
        id: String::new(),
        message_id: ids(root.header::<MESSAGE_ID>()).into_iter().next(),
        in_reply_to: ids(root.header::<IN_REPLY_TO>()),
        flags: Default::default(),
        subject: header::unstructured(root, "Subject").unwrap_or_default(),
        from: addresses(root.header::<FROM>()),
        to: addresses(root.header::<TO>()),
        date,
        size: raw.len() as u64,
        has_attachment: with_attachment.then(|| root.body_lists().has_attachment()),
    }
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
    fn an_id_normalises_the_same_way_wherever_it_came_from() {
        // NOTE: the pairing a reply leans on, the parent's `message_id`
        // and the child's `in_reply_to` entry comparing equal.
        assert_eq!(
            parse_message_ids(" <a@x.org> "),
            [normalize_message_id("<a@x.org>").unwrap()]
        );
    }
}
