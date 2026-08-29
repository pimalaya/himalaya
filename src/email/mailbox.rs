//! # Mailbox
//!
//! A mailbox shared across all protocols.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A mailbox, also known as a folder.
///
/// Strictly least-common-denominator: what is not first-class in every
/// protocol, an IMAP delimiter, a JMAP role, a Maildir path, is reached
/// through the protocol-specific subcommands instead.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Mailbox {
    /// The identifier a follow-up command names the mailbox by.
    ///
    /// JMAP exposes an opaque id of its own, where IMAP, Maildir and
    /// m2dir repeat [`Self::name`].
    pub id: String,
    /// The human-readable name.
    pub name: String,
    /// Total message count, `None` when counts were not asked for or the
    /// backend cannot answer cheaply.
    #[serde(default)]
    pub total: Option<u64>,
    /// Unread message count, `None` on the same terms as [`Self::total`].
    #[serde(default)]
    pub unread: Option<u64>,
}

/// Special-use role of a mailbox.
///
/// Mirrors the IANA JMAP mailbox roles and the RFC 6154 IMAP SPECIAL-USE
/// attributes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MailboxRole {
    /// The mailbox new mail arrives in.
    Inbox,
    /// The mailbox archived messages are kept in.
    Archive,
    /// The mailbox unsent messages are kept in.
    Drafts,
    /// The mailbox flagged messages are gathered in.
    Flagged,
    /// The mailbox important messages are gathered in.
    Important,
    /// The mailbox junk is gathered in.
    Junk,
    /// The mailbox sent messages are kept in.
    Sent,
    /// The mailbox deleted messages are kept in.
    Trash,
    /// A role no registry knows, kept verbatim.
    Other(String),
}

impl MailboxRole {
    /// Reads a role off its wire spelling, one leading `\` stripped and
    /// case insensitive.
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
