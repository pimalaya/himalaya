---
cairn: tasks
change: a-listed-mailbox-is-not-created-again
---

# Tasks

- [x] src/pimdir/backend.rs: `held_mailbox` and `creation_queued`, checked in `create_mailbox` once the performer is known; refusal coded `item-duplicate` naming the listed mailbox.
- [x] src/error.rs: `ItemDuplicate` says it covers a mailbox.
- [x] Test `a_mailbox_the_store_lists_is_not_created_again`: a top-level and a child mailbox the source syncs are refused, another source's mailbox of that name is not, a second creation queued is refused; nothing queued by a refusal. It fails without the checks.
- [x] Fold into cairn/spec/backends.md and the code table of cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
