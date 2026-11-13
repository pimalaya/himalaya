//! # Mailbox
//!
//! A mailbox shared across all protocols.

use std::fmt;

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
#[serde(rename_all = "camelCase")]
pub struct Mailbox {
    /// The identifier a follow-up command names the mailbox by.
    ///
    /// JMAP exposes an opaque id of its own, where IMAP and Maildir
    /// repeat [`Self::name`].
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
#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "pimdir"
))]
#[derive(Clone, Debug, Default)]
pub struct MailboxIndex(pub Vec<Mailbox>);

#[cfg(any(
    feature = "imap",
    feature = "jmap",
    feature = "gmail",
    feature = "msgraph",
    feature = "pimdir"
))]
impl MailboxIndex {
    /// Maps a mailbox id, name or role onto its id.
    ///
    /// A known id passes through, then a name match, then a role match,
    /// either erroring when several mailboxes match. An unknown
    /// value goes back as it is, so the backend surfaces the error or
    /// takes it as a native name.
    #[cfg(any(
        feature = "imap",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph"
    ))]
    pub fn resolve(&self, mailbox: &str) -> Result<String> {
        if self.0.iter().any(|m| m.id == mailbox) {
            return Ok(mailbox.to_string());
        }

        let mut named = self.0.iter().filter(|m| m.name == mailbox);

        if let Some(first) = named.next() {
            let others: Vec<&str> = named.map(|m| m.id.as_str()).collect();

            if !others.is_empty() {
                bail!(
                    "Several mailboxes are named `{mailbox}` ({}, {}): pass the mailbox id",
                    first.id,
                    others.join(", ")
                );
            }

            return Ok(first.id.clone());
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

/// The path of a mailbox named `name` under `parent`, for the backends
/// naming the hierarchy by path (IMAP, Gmail, the local stores).
#[cfg(any(
    feature = "imap",
    feature = "gmail",
    feature = "maildir",
    feature = "mbox"
))]
pub fn child_path(parent: &str, delimiter: char, name: &str) -> String {
    format!("{parent}{delimiter}{name}")
}

/// The path `mailbox` takes once its last level is renamed `name`, its
/// parent kept.
#[cfg(any(
    feature = "imap",
    feature = "gmail",
    feature = "maildir",
    feature = "mbox"
))]
pub fn renamed_path(mailbox: &str, delimiter: char, name: &str) -> String {
    match mailbox.rsplit_once(delimiter) {
        Some((parent, _)) => child_path(parent, delimiter, name),
        None => name.to_string(),
    }
}

/// Refuses a mailbox NAME no backend can take: an empty or blank one, or
/// one holding a control character.
pub fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("Mailbox name must not be empty");
    }

    if name.chars().any(char::is_control) {
        bail!(
            "Mailbox name `{}` must not hold a control character",
            name.escape_debug()
        );
    }

    Ok(())
}

/// Refuses a NAME holding `delimiter`, the backend's hierarchy separator,
/// NAME being one level of the hierarchy.
#[cfg(any(
    feature = "imap",
    feature = "gmail",
    feature = "maildir",
    feature = "mbox"
))]
pub fn validate_level(name: &str, delimiter: char) -> Result<()> {
    if name.contains(delimiter) {
        bail!(
            "Mailbox name `{name}` must not hold `{delimiter}`, the hierarchy delimiter: it names one level of the hierarchy"
        );
    }

    Ok(())
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

    #[test]
    fn a_name_is_not_blank_and_holds_no_control_character() {
        assert!(validate_name("Été 2026").is_ok());
        for name in ["", " ", "a\nb", "a\u{7f}"] {
            assert!(validate_name(name).is_err(), "{name:?}");
        }
    }

    #[cfg(feature = "imap")]
    #[test]
    fn a_level_holds_no_delimiter() {
        assert!(validate_level("a.b", '/').is_ok());
        assert!(validate_level("a.b", '.').is_err());
    }

    #[cfg(any(
        feature = "imap",
        feature = "gmail",
        feature = "maildir",
        feature = "mbox"
    ))]
    #[test]
    fn a_renamed_path_keeps_its_parent() {
        assert_eq!(child_path("Archive", '.', "2026"), "Archive.2026");
        assert_eq!(renamed_path("Archive/2025", '/', "2026"), "Archive/2026");
        assert_eq!(renamed_path("A/B/Old", '/', "New"), "A/B/New");
        assert_eq!(renamed_path("Top", '/', "Renamed"), "Renamed");
    }

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
