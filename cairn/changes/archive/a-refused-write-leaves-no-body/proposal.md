---
cairn: change
id: a-refused-write-leaves-no-body
status: landed
created: 2026-10-30
---

# A refused pimdir write leaves no body behind

## Why

`add_message` and `send_message` stage the body first (`PimdirProducer::stage_body`), then run the gate and the duplicate check while enqueueing. A write refused there (an `item-duplicate` add, a send whose performer does not declare the copy it is asked for) leaves its body unpinned in the blob tree until a collector runs. io-pimdir's waves of 2026-10-30 (log what-the-app-ports-found) added `PimdirProducer::enqueue_with_body_checked`: the gate and, for an add, the duplicate check run on the bytes in hand, and the body is written and the row appended in one transaction, answering the partial supports the gate passed the action with.

The same waves changed `PimdirBlobs::reader`, `PimdirDrainReport`, `PimdirAction::Update.if_match` and `enqueue_with_body`; Himalaya uses none of them (it streams no blob, drains nothing and never edits a body it read), so it built against them unchanged. The new codes it can meet as a reader or producer are relayed (`store-foreign`); `collection-queued`, `collection-role` and `cutoff-invalid` come from owner calls Himalaya never makes.

## What

- `add_message` and `send_message` enqueue through `enqueue_with_body_checked`, warning on its partial supports; the separate `check` and `stage_body` calls go.
- `store-foreign` joins the relayed pimdir codes.
