---
cairn: tasks
change: mailbox-role
---

# Tasks

## Shared type

- [x] Add `role: Option<MailboxRole>` to `Mailbox` (src/email/mailbox.rs), drop the doc line saying a JMAP role is out of the shared shape
- [x] Serialize `MailboxRole` as a plain string, `Other` included; add `All` and `Subscribed` from the IANA registry
- [x] Add a ROLE column to the `mailbox list` table and its color option

## Backends

- [x] JMAP: map `JmapMailboxRole` to `MailboxRole`
- [x] Gmail: move the system-label table out of the wizard into the backend
- [x] Graph: resolve the well-known names to folder ids in one `$batch` (added to io-msgraph) and tag the matching folders
- [x] IMAP: `INBOX` maps to `Inbox`, everything else `None`
- [x] Maildir, m2dir, pimdir: `None` natively

## Alias overlay and resolution

- [x] `Account::apply_role_aliases` overlays `mailbox.alias.<role>` onto the listed roles
- [x] `MailboxIndex` resolves id, then name, then role for the JMAP, Gmail and Graph clients; `EmailClient::role_mailbox_id` replaces `native_trash`
- [x] `-m` resolution stays alias then client, an omitted `-m` becoming the `inbox` alias or role (no more error)
- [x] Error when a requested role matches more than one mailbox, naming them
- [x] `message delete`: `mailbox.alias.trash` before the backend's trash role

## Wizard

- [x] Stop pre-filling aliases, delete src/wizard/mailbox.rs

## Wrap-up

- [x] Point imap-special-use-aliases at `Mailbox.role` instead of wizard aliases
- [x] Update config.sample.toml, README, MIGRATION and CHANGELOG
- [x] `nix develop --command cargo test`, clippy, `cargo fmt`
- [x] Fold the delta, write the log entry

## Follow-ups outside this repo

- [x] Release io-msgraph with `$batch` (0.3.1), then drop the `[patch.crates-io]` path entry from Cargo.toml
- [ ] himalaya-tui (src/tui/update.rs name match) and himalaya-vim read `role`
- [ ] Reply on himalaya#743
