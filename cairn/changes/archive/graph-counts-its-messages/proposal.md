---
cairn: change
id: graph-counts-its-messages
status: landed
created: 2026-11-15
---

# Graph counts a folder's messages

## Why

Task H15 of the family's cli-alignment-2026-11-15 plan, a defect live testing found on 2026-11-15: on the Microsoft test account, `mailbox list --counts` printed `total: 835` for Deleted Items, whose `envelope list` listed 26 and neverest's Graph delta found 26. Himalaya read the folder's `totalItemCount` and `unreadItemCount`, which count every item of the folder, the 809 deleted contacts and events included. io-msgraph 0.9 serves the message count: `MsgraphMessagesListParams::count` asks `$count=true` and `MsgraphPage::count` reads `@odata.count`.

## What

With `--counts`, Graph counts each folder's messages through `$batch`, one request per count, sent as the writes and lookups send theirs: `/messages?$select=id&$top=1&$count=true` for `total`, the same filtered on `isRead eq false` for `unread`, reading `@odata.count`. A folder whose item count, or unread item count, is 0 prints that 0 without a request, no message beating it. A count Graph refuses, answers without `@odata.count`, or leaves out of the answer lists the folder without counts beside a `mailbox-unreadable` entry, as the other backends do. Without `--counts` nothing changes.
