---
cairn: change
change: pimdir-queue-row
---

# Delta

## ADDED Requirements

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"queueId": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyQueueId": i64` (a copy queued beside the send, for an owner declaring no capabilities) for a send. The shared commands SHALL keep printing no queue detail.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands: `{"queueId", "state": "pending" | "parked" | "gone", "collection"?, "kind"?, "attempts"?, "error"?}`, `error` being why a parked row was given up on. A row the queue no longer holds, or one anchored on another account's collection, SHALL read as `gone`: applied, or cancelled.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"queueId":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"gone"`
