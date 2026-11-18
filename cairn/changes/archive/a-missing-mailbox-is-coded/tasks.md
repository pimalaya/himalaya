---
cairn: tasks
change: a-missing-mailbox-is-coded
---

# Tasks

- [x] src/error.rs: `unknown_mailbox`, the code kept with the server's answer.
- [x] src/imap/backend.rs: `explain` over every mailbox-addressing command, `select_mailbox`, `names_no_mailbox`, `missing_mailbox`; the `STATUS` item error.
- [x] src/jmap/client.rs, src/gmail/client.rs, src/msgraph/client.rs: the resolvers coded.
- [x] Tests: tests/missing_mailbox.rs; `a_missing_mailbox_is_coded` (JMAP), `a_missing_label_is_coded`, `a_missing_folder_is_coded`; each failing on 3b9f9127.
- [x] Fold into cairn/spec/commands.md and backends.md, CHANGELOG, MIGRATION.
