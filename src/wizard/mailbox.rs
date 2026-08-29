//! # Mailbox discovery
//!
//! Reads the special-use mailboxes of a freshly configured account, best
//! effort.
//!
//! What it finds is folded into the generated configuration as
//! `mailbox.alias` entries, so the shared commands get an implicit
//! default mailbox and known targets without anyone hand-editing backend
//! ids.
//!
//! It reuses the connection the account test opened, never a second one,
//! and never fails the wizard: an inconclusive listing simply yields
//! fewer aliases.

use std::collections::HashMap;

/// The IMAP inbox alias.
///
/// `INBOX` is the reserved name every RFC 3501 server exposes, so it is
/// always safe to pin.
///
/// TODO: the other roles wait on io-imap issuing a `LIST RETURN
/// (SPECIAL-USE)`, which imap-codec does not support yet, a plain `LIST`
/// advertising the attributes on some servers alone.
#[cfg(feature = "imap")]
pub fn imap_aliases() -> HashMap<String, String> {
    HashMap::from([("inbox".to_string(), "INBOX".to_string())])
}

/// The Gmail special-use aliases, keyed by the fixed system-label ids.
///
/// A system label is universal across every Gmail account and is the very
/// id the API addresses, so no live listing is needed. There is no archive
/// label, archiving being the loss of the `INBOX` one.
#[cfg(feature = "gmail")]
pub fn gmail_aliases() -> HashMap<String, String> {
    [
        ("inbox", "INBOX"),
        ("sent", "SENT"),
        ("drafts", "DRAFT"),
        ("trash", "TRASH"),
        ("junk", "SPAM"),
        ("flagged", "STARRED"),
        ("important", "IMPORTANT"),
    ]
    .into_iter()
    .map(|(key, id)| (key.to_string(), id.to_string()))
    .collect()
}

/// The Microsoft Graph special-use aliases, keyed by the well-known folder
/// names.
///
/// A well-known name is a stable Graph contract accepted in place of a
/// folder id, so no live listing is needed.
#[cfg(feature = "msgraph")]
pub fn msgraph_aliases() -> HashMap<String, String> {
    [
        ("inbox", "inbox"),
        ("sent", "sentitems"),
        ("drafts", "drafts"),
        ("trash", "deleteditems"),
        ("junk", "junkemail"),
        ("archive", "archive"),
    ]
    .into_iter()
    .map(|(key, name)| (key.to_string(), name.to_string()))
    .collect()
}

/// Maps the authoritative RFC 8621 mailbox roles onto alias keys, keyed by
/// the opaque mailbox id.
///
/// Best effort: a failed `Mailbox/get`, or a mailbox with no role, is
/// skipped, the caller having validated the connection already.
#[cfg(feature = "jmap")]
pub fn jmap_aliases(client: &mut crate::jmap::client::JmapClient) -> HashMap<String, String> {
    use io_jmap::rfc8621::mailbox::{JmapMailboxRole, get::JmapMailboxGetOptions};

    let Ok(output) = client.mailbox_get(JmapMailboxGetOptions {
        ids: None,
        properties: None,
    }) else {
        return HashMap::new();
    };

    let mut aliases = HashMap::new();

    for mailbox in output.mailboxes {
        let Some(id) = mailbox.id else {
            continue;
        };

        let key = match mailbox.role {
            Some(JmapMailboxRole::Inbox) => "inbox",
            Some(JmapMailboxRole::Archive) => "archive",
            Some(JmapMailboxRole::Drafts) => "drafts",
            Some(JmapMailboxRole::Flagged) => "flagged",
            Some(JmapMailboxRole::Important) => "important",
            Some(JmapMailboxRole::Junk) => "junk",
            Some(JmapMailboxRole::Sent) => "sent",
            Some(JmapMailboxRole::Trash) => "trash",
            _ => continue,
        };

        aliases.insert(key.to_string(), id);
    }

    aliases
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "gmail")]
    #[test]
    fn gmail_aliases_map_the_system_labels() {
        let aliases = super::gmail_aliases();
        assert_eq!(aliases["inbox"], "INBOX");
        assert_eq!(aliases["sent"], "SENT");
        assert_eq!(aliases["drafts"], "DRAFT");
        assert_eq!(aliases["trash"], "TRASH");
        assert_eq!(aliases["junk"], "SPAM");
    }

    #[cfg(feature = "msgraph")]
    #[test]
    fn msgraph_aliases_map_the_well_known_names() {
        let aliases = super::msgraph_aliases();
        assert_eq!(aliases["inbox"], "inbox");
        assert_eq!(aliases["sent"], "sentitems");
        assert_eq!(aliases["drafts"], "drafts");
        assert_eq!(aliases["trash"], "deleteditems");
        assert_eq!(aliases["junk"], "junkemail");
        assert_eq!(aliases["archive"], "archive");
    }
}
