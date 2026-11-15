---
cairn: log
change: graph-counts-its-messages
landed: 2026-11-15
---

# Graph counts a folder's messages

Task H15 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15: the Microsoft test account's Deleted Items printed `total: 835` under `mailbox list --counts`, where its listing held 26 messages, Graph's `totalItemCount` counting the deleted contacts and events too.

## What landed

- src/msgraph/backend.rs: `list_mailboxes` answers an `Outcome`. With `--counts`, `count_requests` asks each folder's `/messages?$select=id&$top=1&$count=true`, and the same filtered on `isRead eq false`, through `batch_all`; `counts_from` reads `@odata.count` (io-msgraph's `MsgraphPage::count`). A folder whose item count or unread item count is 0 prints that 0 unasked. A count refused, carrying no `@odata.count` or left out of the answer makes a `mailbox-unreadable` entry, the folder's counts `null`.
- src/msgraph/client.rs `MsgraphClient::over`, a test constructor over any stream; src/shared/client.rs and the folder index follow the `Outcome`.
- Tests on a scripted Graph server: `a_folder_counts_its_messages_not_its_items` (a folder answering `totalItemCount` 835 whose `/messages` counts 26 prints 26, its unread 0 unasked; the old code printed 835 and sent no count request), `a_refused_count_leaves_the_folder_uncounted`, `an_uncounted_listing_asks_no_count`.
- Not run against Graph: live tests are the live testing session's.

## Capabilities moved

- [backends](../spec/backends.md): *Mailbox counts are asked for* and its scenario *A Graph folder holding contacts*.
