---
cairn: change
id: mailbox-role
status: landed
created: 2026-09-29
---

# Expose the mailbox role on the shared API

## Why

A UI needs to target the inbox, sent, drafts or trash mailbox without guessing from names (himalaya#743). Today the only source is `mailbox.alias.*`, which the wizard pre-fills from native roles it already knows (JMAP roles, Gmail system labels, Graph well-known names). The shared `Mailbox` carries no role, so himalaya-tui matches `inbox` by name, which breaks on JMAP opaque ids and localized names.

There is no default mailbox as such, only roles. Most backends expose them natively; the alias is the fallback for backends that cannot.

## What

- `Mailbox` gains `role: Option<MailboxRole>`, filled by each backend from its native source and shown in `mailbox list` (table and `--json`).
- Native sources: JMAP `role`; Gmail fixed system-label ids; Graph well-known folder names resolved to ids; IMAP `INBOX` only, every other mailbox `None` until LIST-EXTENDED lands in imap-codec (see imap-special-use-aliases). Maildir, m2dir and pimdir have no native role.
- Aliases stay, with a new precedence: an explicit `mailbox.alias.<role>` overrides the native role, so a user can fix a server that mislabels. Backends without native roles (Maildir, m2dir, pimdir) get roles from aliases only, and `None` otherwise.
- `-m <name>` resolves alias, then native role, then passes the value verbatim. An omitted `-m` resolves the `inbox` role, so a missing `inbox` alias no longer errors on JMAP, Gmail, Graph and IMAP. Two mailboxes sharing the requested role is an error naming both.
- The wizard stops pre-filling aliases for backends with native roles, keeping only what a backend cannot discover.

## Out of scope

IMAP roles beyond `INBOX` (blocked on duesee/imap-codec LIST-EXTENDED, then io-imap). Guessing Maildir roles from Maildir++ folder names.
