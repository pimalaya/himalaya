---
cairn: tasks
change: the-local-stores-add-a-message
---

# Tasks

- [x] Rename src/maildir/message/save.rs and src/mbox/message/save.rs to add.rs, the types to `*MessageAdd*`, the variants to `Add` with `alias = "save"`.
- [x] Rename the schema keys to `himalaya-maildir-message-add` and `himalaya-mbox-message-add`.
- [x] Tests `a_maildir_message_is_added` and `an_mbox_message_is_added` (src/cli.rs): `add` and `save` parse to `Add`, the help lists `add` and not `save`; tests/local_stores.rs calls `mbox message add`.
- [x] Fold into cairn/spec/backends.md; CHANGELOG (**BREAKING**) and MIGRATION.md; the log entry.
