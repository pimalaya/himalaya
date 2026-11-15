---
cairn: tasks
change: graph-reads-its-unread-count
---

# Tasks

- [x] src/msgraph/backend.rs: `mailbox_from` reads `unread` off the folder, `count_requests` asks the total alone (one request per folder holding items), `counts_from` fills the totals.
- [x] Tests on the scripted Graph server: `a_folder_counts_its_messages_not_its_items` (totalItemCount 835, unreadItemCount 5, `@odata.count` 26: prints 26 and 5, no `isRead` request; the old code asked one and failed the folder on the script), `a_refused_count_leaves_the_folder_uncounted`.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; the log entry.
