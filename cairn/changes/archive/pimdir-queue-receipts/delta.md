---
cairn: change
change: pimdir-queue-receipts
---

# Delta

## MODIFIED Requirements

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"queueId": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyQueueId": i64` (a copy queued beside the send, for an owner declaring no capabilities) for a send. The shared commands SHALL keep printing no queue detail.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4): `{"queueId", "state": "pending" | "parked" | "applied" | "unknown", "collection"?, "kind"?, "attempts"?, "error"?, "appliedAt"?, "seq"?}`. A pending or parked row carries `collection`, `kind` and `attempts`, a parked one `error` too; an applied one carries `collection`, `appliedAt` and, for an add, `seq`, the public id of the message it created. A row neither queued nor in the store's receipts (kept seven days), or one anchored on another account's collection, SHALL read as `unknown`: cancelled, or applied long ago.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"queueId":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `seq` the message is read by

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"queueId": i64, "name": text, "parent"?: collection, "source": id}`. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A name the server already holds is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `queueId` and `"source":"imap"`

