---
cairn: log
change: a-duplicate-mailbox-names-its-noun
landed: 2026-11-15
---

# A duplicate mailbox names its noun

Ruling D17 of the family's cli-alignment-2026-11-15 plan: a create naming a mailbox that exists fails `mailbox-duplicate` on every backend that can tell, where Himalaya failed `item-duplicate` on pimdir and uncoded elsewhere.

## What landed

- src/error.rs: the code `mailbox-duplicate`, `mailbox_duplicate` and `existing_mailbox`; `item-duplicate` codes a message alone.
- pimdir (src/pimdir/backend.rs): a listed or queued mailbox; Maildir (src/maildir/client.rs `refuse_existing`, for `mailbox create` and `maildir create`): a folder at the path, a file in the way uncoded; mbox (src/mbox/backend.rs, for `mailbox create` and `mbox create`): io-mbox's `AlreadyExists` on a file, a directory uncoded.
- IMAP (src/imap/backend.rs `create_refusal`): `ALREADYEXISTS`, or a refusal with no code while `LIST` shows the mailbox; JMAP (src/jmap/backend.rs `holds_sibling`): a refusal coded no other way while a sibling holds the name; Gmail: a 409 on the label create; Graph (src/msgraph/backend.rs): a 409 `ErrorFolderExists` at the top, or a child of that name the parent holds, looked up before the folder is created at the top.
- Tests: one per backend over a temp store or a scripted server, and tests/local_stores.rs `a_mailbox_created_twice_is_a_duplicate` through the binary.

## Capabilities moved

- [commands](../spec/commands.md): *Unified mailbox writes* codes a duplicate create per backend, with the scenario *A mailbox created twice*; *The raw Maildir folder commands guard the store* codes an existing folder; the code table gains `mailbox-duplicate` and narrows `item-duplicate` to a message.
- [backends](../spec/backends.md): *pimdir creates a mailbox by queueing an intent* refuses with `mailbox-duplicate`.
