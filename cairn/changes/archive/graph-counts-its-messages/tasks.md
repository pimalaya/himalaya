---
cairn: tasks
change: graph-counts-its-messages
---

# Tasks

- [x] src/msgraph/backend.rs: `list_mailboxes` answers an `Outcome`, counting through `count_requests` and `counts_from`; src/msgraph/client.rs and src/shared/client.rs follow.
- [x] Tests on a scripted Graph server: `a_folder_counts_its_messages_not_its_items` (totalItemCount 835, `@odata.count` 26), `a_refused_count_leaves_the_folder_uncounted`, `an_uncounted_listing_asks_no_count`.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; the log entry.
