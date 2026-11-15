---
cairn: delta
change: graph-counts-its-messages
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Mailbox counts are asked for
`mailbox list` SHALL fill a mailbox's `total` and `unread` with `--counts` only, both `null` otherwise, Graph returning item counts inline included. With `--counts`, IMAP SHALL issue one `STATUS` per mailbox, JMAP read them from `Mailbox/get`, Gmail issue one `labels.get` per label, Graph count the folder's messages, mbox sync the index of each file, and pimdir count the items and the unread mail of the store. Maildir SHALL leave both `null`. A Graph folder's `totalItemCount` and `unreadItemCount` count its contacts and events too, so Graph SHALL count through `$batch`, sent as "Graph batches four requests at a time" says, the `@odata.count` of the folder's `/messages` asked with `$count=true` over a one-message page, and of the same filtered on `isRead eq false` for `unread`; a folder whose item count is 0, or unread item count 0, SHALL print that 0 unasked.

#### Scenario: A Graph folder holding contacts
- GIVEN a Graph Deleted Items folder answering `totalItemCount` 835 and `unreadItemCount` 0, whose `/messages` counts 26
- WHEN `mailbox list --counts` runs
- THEN it prints `total` 26 and `unread` 0, asking no unread count

## REMOVED Requirements

None.
