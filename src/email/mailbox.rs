//! Mailbox shared across all protocols.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mailbox (a.k.a. folder).
///
/// Strict least-common-denominator shape: only fields that are
/// first-class in every protocol the CLI targets (IMAP, JMAP, Maildir,
/// m2dir). Protocol-specific data (IMAP delimiter and SPECIAL-USE
/// attributes, JMAP role and rights, Maildir path, …) is intentionally
/// absent; for these, use the protocol-specific subcommands.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Mailbox {
    /// Backend-specific identifier.
    ///
    /// JMAP exposes a real opaque ID; for IMAP, Maildir and m2dir this
    /// is the same as [`Self::name`]. Use this when issuing follow-up
    /// commands that refer to the mailbox.
    pub id: String,

    /// Human-readable mailbox name.
    pub name: String,

    /// Total number of messages, when the caller requested counts.
    /// `None` when the backend was not asked or cannot answer cheaply.
    #[serde(default)]
    pub total: Option<u64>,

    /// Number of unread messages, when the caller requested counts.
    /// `None` when the backend was not asked or cannot answer cheaply.
    #[serde(default)]
    pub unread: Option<u64>,
}

/// Special-use role of a mailbox.
///
/// Mirrors the IANA JMAP mailbox roles and the IMAP SPECIAL-USE
/// attributes (RFC 6154). [`MailboxRole::Other`] holds any value that
/// does not match a known role.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MailboxRole {
    Inbox,
    Archive,
    Drafts,
    Flagged,
    Important,
    Junk,
    Sent,
    Trash,
    Other(String),
}

impl MailboxRole {
    pub fn parse(raw: &str) -> Self {
        match raw.trim_start_matches('\\').to_ascii_lowercase().as_str() {
            "inbox" => Self::Inbox,
            "archive" => Self::Archive,
            "drafts" => Self::Drafts,
            "flagged" => Self::Flagged,
            "important" => Self::Important,
            "junk" | "spam" => Self::Junk,
            "sent" => Self::Sent,
            "trash" => Self::Trash,
            _ => Self::Other(raw.into()),
        }
    }
}
