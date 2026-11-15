---
cairn: tasks
change: a-move-onto-its-own-mailbox-changes-nothing
---

# Tasks

- [x] src/mbox/backend.rs `move_message_ids`: a no-op when `same_mbox` (one file, however spelled); src/imap/backend.rs `move_messages`: a no-op past the `UID SEARCH` when `same_mailbox` (`INBOX` in any case); src/pimdir/backend.rs `move_messages`: a no-op past the lookup.
- [x] tests/local_stores.rs `an_mbox_move_onto_its_own_file_changes_nothing`: two messages, the unified and the raw move exit 0, the file byte-identical; IMAP `a_move_onto_its_own_mailbox_changes_nothing` on the scripted server, with and without `MOVE`; pimdir `a_refile_or_delete_stages_the_ids_it_found` stages nothing for a move onto `imap/INBOX`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; the log entry.
