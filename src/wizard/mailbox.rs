//! Best-effort special-use mailbox discovery for the wizard.
//!
//! The discovered roles are folded into the generated config as
//! `mailbox.alias.*`, so shared commands have an implicit default mailbox
//! (the `inbox` alias) and known Sent/Drafts/Trash/… targets without the
//! user hand-editing backend ids. Discovery reuses the connection opened
//! for the account test (never a second one) and never fails the wizard:
//! an inconclusive listing just yields fewer aliases.

use std::collections::HashMap;

/// The IMAP inbox alias.
///
/// `INBOX` is a reserved, case-insensitive mailbox name every IMAP server
/// exposes (RFC 3501), so it is always safe to pin. The other special-use
/// roles are deferred until io-imap can issue LIST `RETURN (SPECIAL-USE)`
/// (RFC 6154): imap-codec has no support yet (duesee/imap-codec#350), and
/// a plain LIST advertises the attributes only on some servers.
#[cfg(feature = "imap")]
pub fn imap_aliases() -> HashMap<String, String> {
    HashMap::from([("inbox".to_string(), "INBOX".to_string())])
}

/// The Gmail special-use aliases, keyed by Gmail's fixed system-label ids.
///
/// The system labels (`INBOX`, `SENT`, …) are universal across every Gmail
/// account and are the very ids the API addresses, so — like the IMAP
/// `INBOX` — no live listing is needed. Gmail has no archive label
/// (archiving just drops the `INBOX` label), and `inbox` is always present
/// as the default mailbox.
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

/// The Microsoft Graph special-use aliases, keyed by Graph's well-known
/// folder names.
///
/// The well-known names (`inbox`, `sentitems`, …) are a stable Graph
/// contract accepted directly in place of a folder id, so — like the IMAP
/// `INBOX` — no live listing is needed. `inbox` is always present as the
/// default mailbox.
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

/// Maps the JMAP mailbox roles (RFC 8621, authoritative) to alias keys,
/// keyed by the opaque mailbox id.
///
/// Best-effort: a failed `Mailbox/get` (or a role-less mailbox) is simply
/// skipped, since the connection was already validated by the caller.
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
