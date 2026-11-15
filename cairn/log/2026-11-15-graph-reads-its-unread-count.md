---
cairn: log
change: graph-reads-its-unread-count
landed: 2026-11-15
---

# Graph reads its unread count

Task H17 of the family's cli-alignment-2026-11-15 plan: Graph's `unreadItemCount` proved right live, so the unread count of `mailbox list --counts` reads it again and asks nothing more.

## What landed

- src/msgraph/backend.rs: `mailbox_from` takes `unread` from the folder's `unreadItemCount` under `--counts`; `count_requests` asks one `$count` per folder holding items, for `total`, the `$batch` ids being the folder indexes; `counts_from` fills `total`, a refused count leaving both counts `null` beside a `mailbox-unreadable` entry.
- src/shared/http_script.rs: the scripted HTTP stream the Graph tests used, shared with the Gmail ones.
- Tests: `a_folder_counts_its_messages_not_its_items` now answers `unreadItemCount` 5 and expects it unasked (the old code sent an `isRead` count the script did not answer, failing the folder), `a_refused_count_leaves_the_folder_uncounted` asserts no request filters on `isRead`.
- Not run against Graph: live tests are the live testing session's.

## Capabilities moved

- [backends](../spec/backends.md): *Mailbox counts are asked for* and its scenario *A Graph folder holding contacts*.
