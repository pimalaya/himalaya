---
cairn: log
change: pimdir-mailbox-roles
date: 2026-10-05
---

# A pimdir mailbox carries its role

The pimdir backend listed every mailbox with `role: None`: the store had nowhere to keep one. pimdir draft-04 adds `collections.role`, which neverest fills from what each server states (IMAP special-use, Graph well-known folders, Gmail system labels), and io-pimdir `ef8eae0` reads it.

## What landed

- Cargo.toml, Cargo.lock: io-pimdir by git rev `ef8eae0`.
- src/pimdir/backend.rs: `list_mailboxes` maps `PimdirCollection::role` through `MailboxRole::parse`, so `mailbox list --json` carries it and a role name resolves to its mailbox (`Mailboxes::with_role`) as on the other backends.
- Unit test `a_mailbox_lists_the_role_its_store_records`.

## Verification

`cargo test --all-features`: 181 passed. `cargo clippy --all-features --all-targets` clean.
