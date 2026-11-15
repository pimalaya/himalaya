---
cairn: log
change: a-move-onto-its-own-mailbox-changes-nothing
landed: 2026-11-15
---

# A move onto its own mailbox changes nothing

Task H19 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (mbox.md, H-1): a move onto its own mbox rewrote the file then failed exit 1.

## What landed

- src/mbox/backend.rs: `move_message_ids` returns the ids found, writing nothing, when `same_mbox` says source and target are one file (equal, or one once canonicalized: a name and an absolute path).
- src/imap/backend.rs: `move_messages` looks the UIDs up (`UID SEARCH`) and returns them when `same_mailbox` (`INBOX` in any case), with or without `MOVE`.
- src/pimdir/backend.rs: `move_messages` counts the ids the mailbox holds and stages nothing when both name one collection.
- What the others do, read in the code: JMAP (`from == to` before any `Email/set`), Gmail (`ids_in` then nothing), Graph (`ids_in_folder` then nothing) and Maildir (`source.path() != target.path()` guarding the renames) already changed nothing.
- Tests: tests/local_stores.rs `an_mbox_move_onto_its_own_file_changes_nothing` (unified `-f A -t A` and raw `-m A -t <absolute A>`, exit 0, `count` 1 and the id, the file byte-identical), IMAP `a_move_onto_its_own_mailbox_changes_nothing`, pimdir `a_refile_or_delete_stages_the_ids_it_found`; each fails on the old code (exit 1 with a rewritten file, a `UID MOVE` or `UID COPY` sent, a queued move).

## Capabilities moved

- [commands](../spec/commands.md): *A command given message ids says how many it found*, a move onto its own mailbox changing nothing on every backend, and its scenario *An mbox move onto its own file*.
