---
cairn: change
id: a-pimdir-role-names-its-mailbox
status: landed
created: 2026-10-30
---

# A role name addresses its pimdir mailbox

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, should-fix 2) verified that the role names `mailbox list` shows on a pimdir account address nothing. `EmailClient::role_mailbox_id` returns `Ok(None)` for pimdir (src/shared/client.rs:433), `resolve_mailbox_id` passes the value through (src/shared/client.rs:533-543), and `MailboxIndex` is gated to JMAP, Gmail and Graph (src/email/mailbox.rs:44-48). So `envelope list` without `-m` fails with "Mailbox `inbox` not found", `-m inbox` fails the same way, and `message delete` without `mailbox.alias.trash` fails with "Cannot determine the trash mailbox" though a trash-role collection exists. CHANGELOG.md:26 and cairn/log/2026-10-05-pimdir-mailbox-roles.md claim the opposite; cairn/spec/backends.md ("Mailbox role") and config.sample.toml say pimdir has no roles.

## What

- The pimdir backend resolves a mailbox as its collection id first, then as a role name (`inbox`, `sent`, `trash`, ...) through the roles the store records (`collections.role`), the mailbox carrying it; several mailboxes of the account carrying one role is refused naming them, as on JMAP. Names are not matched: a pimdir mailbox is its collection id, and two sources may hold a mailbox of the same name.
- `role_mailbox_id` answers from the same roles, so `message delete` finds the trash, and the shared `resolve_mailbox_id` maps a role to its id, so the trash comparison holds however each mailbox was named.
- The `pimdir` subcommands resolve through the same path, `hub_id` being where every pimdir mailbox goes through.
- spec/backends.md ("Mailbox role", "A pimdir mailbox is its collection id"), config.sample.toml and the CHANGELOG say so.
