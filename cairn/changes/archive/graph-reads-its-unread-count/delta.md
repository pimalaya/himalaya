---
cairn: delta
change: graph-reads-its-unread-count
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Mailbox counts are asked for
`mailbox list` SHALL fill a mailbox's `total` and `unread` with `--counts` only, both `null` otherwise, Graph returning item counts inline included. With `--counts`, IMAP SHALL issue one `STATUS` per mailbox, JMAP read them from `Mailbox/get`, Gmail issue one `labels.get` per label, Graph count the folder's messages, mbox sync the index of each file, and pimdir count the items and the unread mail of the store. Maildir SHALL leave both `null`. A Graph folder's `totalItemCount` counts its contacts and events too, so Graph SHALL count `total` through `$batch`, sent as "Graph batches four requests at a time" says, the `@odata.count` of the folder's `/messages` asked with `$count=true` over a one-message page, a folder whose item count is 0 printing that 0 unasked; `unread` SHALL be the folder's `unreadItemCount`, which counts its unread messages alone.

#### Scenario: A Graph folder holding contacts
- GIVEN a Graph Deleted Items folder answering `totalItemCount` 835 and `unreadItemCount` 5, whose `/messages` counts 26
- WHEN `mailbox list --counts` runs
- THEN it prints `total` 26 and `unread` 5, asking one count and none of the unread messages

## REMOVED Requirements

None.
