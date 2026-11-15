---
cairn: tasks
change: the-last-family-lows
---

# Tasks

- [x] X9c src/error.rs (`mailbox_duplicate` takes the name, `existing_mailbox` for pimdir too), src/maildir/client.rs, src/msgraph/backend.rs, src/pimdir/backend.rs; the sentence pinned in `an_existing_folder_is_not_created_again` (Maildir), `an_existing_mbox_is_not_created_again`, `a_mailbox_the_store_lists_is_not_created_again`, `a_create_of_an_existing_folder_is_a_duplicate` (Graph).
- [x] src/pimdir/queue/list.rs, src/pimdir/backend.rs: `attempts: u64`; test `a_row_counts_its_attempts_from_zero`.
- [x] D29 src/shared/attachment/list.rs, download.rs: `short_alias = 'i'`; test `inline_keeps_a_hidden_short_alias`.
- [x] Fold into cairn/spec/commands.md and backends.md; CHANGELOG; MIGRATION.md; the log entry.
