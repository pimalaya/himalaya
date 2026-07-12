//! Envelope shared across all protocols.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::email::{address::Address, flag::Flag};

/// Lightweight summary of a message: enough to display in a list
/// without fetching the full body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
