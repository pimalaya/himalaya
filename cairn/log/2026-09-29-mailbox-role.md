---
cairn: log
change: mailbox-role
landed: 2026-09-29
---

# Mailbox role on the shared API

Asked for by himalaya#743: a UI needs to target the inbox, sent or trash mailbox without matching names, which breaks on JMAP opaque ids and localized names.

`Mailbox` carries an optional `MailboxRole`, serialized as its lowercase wire spelling and shown as a ROLE column. JMAP maps its roles, Gmail its system-label ids, and Graph resolves its well-known folder names to ids in one `$batch` request, added to io-msgraph for this. IMAP marks `INBOX` alone until LIST-EXTENDED lands in imap-codec. Maildir, m2dir and pimdir have no native role.

Aliases became overrides: `mailbox.alias.<role>` sets the role of the mailbox it names in listings and wins over the backend's trash in `message delete`. The JMAP, Gmail and Graph clients resolve `-m` through a cached `MailboxIndex`, by id, then name, then role, so `-m sent` works without an alias and an omitted `-m` falls back to the inbox role instead of failing. A role several mailboxes carry is an error. The wizard, whose aliases only mirrored what the backends now report, no longer writes any.

Himalaya builds against io-msgraph through a `[patch.crates-io]` path entry until io-msgraph is released.

## Capabilities moved

- [backends](../spec/backends.md): *Mailbox role* added.
- [config](../spec/config.md): *Mailbox aliases* resolve roles and no longer require `inbox`.
- [wizard](../spec/wizard.md): *Mailbox alias pre-fill* replaced by *No mailbox alias pre-fill*.
- [provider-quirks](../spec/provider-quirks.md): *IMAP special-use is inbox-only for now* speaks of roles, not wizard aliases.
