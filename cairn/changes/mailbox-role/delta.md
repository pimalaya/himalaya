---
cairn: delta
change: mailbox-role
---

## ADDED Requirements

### Requirement: Mailbox role
A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its table and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids in one `$batch` request, and IMAP marks only `INBOX`. Maildir, m2dir and pimdir have no native role. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role.

### Requirement: No mailbox alias pre-fill
The wizard SHALL NOT pre-fill `mailbox.alias.*`: the backends report their mailbox roles at runtime, and aliases are the user's overrides.

## MODIFIED Requirements

### Requirement: Mailbox aliases
An account MAY map friendly mailbox names to backend-native ids under `[accounts.<name>.mailbox.alias]`. Alias names are case-insensitive on lookup and on storage. `-m/--mailbox` resolves an alias first, then a mailbox role, then passes the value verbatim; a role matching several mailboxes is an error. A shared command that omits `-m/--mailbox` resolves the `inbox` alias, else the `inbox` role. Account-level entries override same-named global entries, and ids are stored verbatim.

### Requirement: IMAP special-use is inbox-only for now
IMAP SHALL mark only the reserved `INBOX` with a mailbox role. Reading the Sent/Drafts/Trash/Junk/Archive roles needs LIST `RETURN (SPECIAL-USE)` (RFC 6154), which io-imap cannot yet issue because upstream imap-codec has no LIST-EXTENDED support. The other IMAP roles are set through `mailbox.alias.<role>` until then.

## REMOVED Requirements

### Requirement: Mailbox alias pre-fill
Replaced by *No mailbox alias pre-fill*.
