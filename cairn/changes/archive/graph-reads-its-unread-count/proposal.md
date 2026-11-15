---
cairn: change
id: graph-reads-its-unread-count
status: landed
created: 2026-11-15
---

# Graph reads its unread count

## Why

Task H17 of the family's cli-alignment-2026-11-15 plan, from the live check of H15 on 2026-11-15: Graph's `unreadItemCount` proved right on the Microsoft test account (Deleted Items read 5 unread, as its envelope listing), unlike its `totalItemCount`, which counts contacts and events. `mailbox list --counts` asked a second `$count` request per folder for the unread messages, which the folder already carries.

## What

With `--counts`, `unread` is the folder's `unreadItemCount` again; `total` keeps the message count of graph-counts-its-messages, one `$count` request per folder holding items. A folder whose count Graph refuses still lists without counts beside its error.
