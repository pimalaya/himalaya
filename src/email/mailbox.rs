//! # Mailbox
//!
//! A mailbox shared across all protocols.

use std::fmt;

#[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A mailbox, also known as a folder.
///
/// Strictly least-common-denominator: what is not first-class in every
/// protocol, an IMAP delimiter, a Maildir path, is reached through the
/// protocol-specific subcommands instead. The role is optional, so a
/// backend without one still fits.
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
    /// The special-use role, from the backend or a `mailbox.alias.<role>`
    /// entry, `None` when neither names one.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub role: Option<MailboxRole>,
    /// Total message count, `None` when counts were not asked for or the
    /// backend cannot answer cheaply.
    #[serde(default)]
    pub total: Option<u64>,
    /// Unread message count, `None` on the same terms as [`Self::total`].
    #[serde(default)]
    pub unread: Option<u64>,
}

/// The mailboxes of a backend minting opaque ids, which names and roles
/// resolve through.
#[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
#[derive(Clone, Debug, Default)]
pub struct MailboxIndex(pub Vec<Mailbox>);

#[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
impl MailboxIndex {
    /// Maps a mailbox id, name or role onto its id.
    ///
    /// A known id passes through, then a name match, then a role match,
    /// which errors when several mailboxes carry the role. An unknown
    /// value goes back as it is, so the backend surfaces the error or
    /// takes it as a native name.
    pub fn resolve(&self, mailbox: &str) -> Result<String> {
        if self.0.iter().any(|m| m.id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some(m) = self.0.iter().find(|m| m.name == mailbox) {
            return Ok(m.id.clone());
        }

        if let Some(role) = MailboxRole::known(mailbox)
            && let Some(id) = self.with_role(&role)?
        {
            return Ok(id);
        }

        Ok(mailbox.to_string())
    }

    /// The id of the mailbox carrying `role`, `None` when none does.
    ///
    /// Errors when several do, picking one silently being how messages
    /// end up in the wrong place.
    pub fn with_role(&self, role: &MailboxRole) -> Result<Option<String>> {
        let mut matches = self.0.iter().filter(|m| m.role.as_ref() == Some(role));

        let Some(first) = matches.next() else {
            return Ok(None);
        };

        let others: Vec<&str> = matches.map(|m| m.id.as_str()).collect();
        if !others.is_empty() {
            bail!(
                "Several mailboxes carry the `{role}` role ({}, {}): pass the mailbox id, or set `mailbox.alias.{role}` in your configuration",
                first.id,
                others.join(", ")
            );
        }

        Ok(Some(first.id.clone()))
    }
}

/// Special-use role of a mailbox.
///
/// Mirrors the IANA IMAP Mailbox Name Attributes registry, shared by the
/// JMAP mailbox roles (RFC 8621) and the IMAP SPECIAL-USE attributes (RFC
/// 6154). Serialized as its lowercase wire spelling.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MailboxRole {
    /// The mailbox new mail arrives in.
    Inbox,
    /// The virtual mailbox holding every message.
    All,
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
    /// The virtual mailbox holding the subscribed ones.
    Subscribed,
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
            "all" => Self::All,
            "archive" => Self::Archive,
            "drafts" => Self::Drafts,
            "flagged" => Self::Flagged,
            "important" => Self::Important,
            "junk" | "spam" => Self::Junk,
            "sent" => Self::Sent,
            "subscribed" => Self::Subscribed,
            "trash" => Self::Trash,
            _ => Self::Other(raw.into()),
        }
    }

    /// Reads a role a user named, `None` for a name no registry knows.
    pub fn known(name: &str) -> Option<Self> {
        match Self::parse(name) {
            Self::Other(_) => None,
            role => Some(role),
        }
    }

    /// The lowercase wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inbox => "inbox",
            Self::All => "all",
            Self::Archive => "archive",
            Self::Drafts => "drafts",
            Self::Flagged => "flagged",
            Self::Important => "important",
            Self::Junk => "junk",
            Self::Sent => "sent",
            Self::Subscribed => "subscribed",
            Self::Trash => "trash",
            Self::Other(raw) => raw,
        }
    }
}

impl fmt::Display for MailboxRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for MailboxRole {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MailboxRole {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::parse(&String::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
    fn index() -> MailboxIndex {
        let mailbox = |id: &str, name: &str, role: Option<MailboxRole>| Mailbox {
            id: id.to_string(),
            name: name.to_string(),
            role,
            total: None,
            unread: None,
        };

        MailboxIndex(vec![
            mailbox("m1", "Inbox", Some(MailboxRole::Inbox)),
            mailbox("m2", "Envoyés", Some(MailboxRole::Sent)),
            mailbox("m3", "Corbeille", Some(MailboxRole::Trash)),
            mailbox("m4", "Poubelle", Some(MailboxRole::Trash)),
        ])
    }

    #[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
    #[test]
    fn index_resolves_ids_then_names_then_roles() {
        let index = index();
        assert_eq!(index.resolve("m2").unwrap(), "m2");
        assert_eq!(index.resolve("Inbox").unwrap(), "m1");
        assert_eq!(index.resolve("sent").unwrap(), "m2");
        assert_eq!(index.resolve("SENT").unwrap(), "m2");
        assert_eq!(index.resolve("drafts").unwrap(), "drafts");
        assert_eq!(index.resolve("unknown").unwrap(), "unknown");
    }

    #[cfg(any(feature = "jmap", feature = "gmail", feature = "msgraph"))]
    #[test]
    fn index_rejects_a_role_several_mailboxes_carry() {
        let err = index().resolve("trash").unwrap_err().to_string();
        assert!(err.contains("m3, m4"), "got: {err}");
    }

    #[test]
    fn role_serializes_as_its_wire_spelling() {
        let json = serde_json::to_string(&[MailboxRole::Junk, MailboxRole::parse("\\Custom")]);
        assert_eq!(json.unwrap(), r#"["junk","\\Custom"]"#);
    }
}
