---
cairn: change
id: pimdir-queue-receipts
status: landed
created: 2026-10-04
---

# A queue row says what it became

## Why

pimdir-queue-row left `queue show` unable to tell an applied row from a cancelled one, nor to give the id of the message an add created: io-pimdir deleted applied rows without a trace. io-pimdir `3a9ffb8` (pimdir draft-04) records a receipt for each applied row, kept seven days, read by `action_status`, and names the `collection-create` intent with `enqueue_collection_create`.

## What

- Pin io-pimdir to `3a9ffb8` by git rev.
- `pimdir queue show` reads `action_status`: `pending`, `parked`, `applied` (with `appliedAt` and, for an add, `seq`) or `unknown`, which replaces `gone`.
- `pimdir mailbox create` queues through `enqueue_collection_create`, its kind, payload and capability now io-pimdir's.
