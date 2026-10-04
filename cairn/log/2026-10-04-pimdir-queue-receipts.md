---
cairn: log
change: pimdir-queue-receipts
landed: 2026-10-04
---

# A queue row says what it became

io-pimdir is pinned to `3a9ffb8` by git rev. `himalaya pimdir queue show <ROW>` reads its `action_status` and prints `{"queueId","state","collection"?,"kind"?,"attempts"?,"error"?,"appliedAt"?,"seq"?}`, the state `pending`, `parked`, `applied` or `unknown`; `gone` is gone. An applied add carries `seq`, the public id of the message it created, for seven days. `himalaya pimdir mailbox create` queues through `enqueue_collection_create`.

Capabilities moved: **backends** (pimdir writes name their queue row; pimdir creates a mailbox by queueing an intent).
