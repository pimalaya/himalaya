---
cairn: log
change: the-local-stores-add-a-message
landed: 2026-11-16
---

# The local stores add a message

Task L18 of the family's cli-alignment-2026-11-15 plan (owner ruling D32): `add` creates an item across the family (pimalaya-cli cairn/spec/commands.md).

## What landed

- src/maildir/message/add.rs and src/mbox/message/add.rs (were save.rs): `maildir message add` and `mbox message add`, `save` a hidden alias of each (src/maildir/message/cli.rs, src/mbox/message/cli.rs); types `MaildirMessageAddCommand`, `MaildirMessageAddOutput`, `MboxMessageAddCommand`, `MboxMessageAddOutput`; the confirmation reads `successfully added`.
- src/json_schema.rs: the keys `himalaya-maildir-message-add` and `himalaya-mbox-message-add`, the `-save` keys v2.2.1 shipped being gone.
- src/cli.rs: `a_maildir_message_is_added` and `an_mbox_message_is_added` (both verbs parse to `Add`, the help lists `add` alone); tests/local_stores.rs calls `mbox message add`.
- MCP unchanged: it exposes the unified API alone.
- CHANGELOG (**BREAKING**, the schema keys) and MIGRATION.md.

## Capabilities moved

- [backends](../spec/backends.md): *Local storage backends* and *mbox backend*.
