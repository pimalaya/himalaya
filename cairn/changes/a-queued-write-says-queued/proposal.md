---
cairn: change
id: a-queued-write-says-queued
status: landed
created: 2026-10-30
---

# A queued write says it is queued

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, should-fix 4) verified that the shared commands claim, on a pimdir account, what has not happened. `message send --save` prints "Message successfully saved and sent" (src/shared/message/handler.rs:119-121) when only a `submit` row was queued, in a store where no source may ever perform it. `message delete` prints "Successfully moved 1 message(s) to the trash" for a staged `move`. Every pimdir write is a row in the store's queue until the sync engine applies it, and the confirmation should say so. The plan (pimdir stable release, Phase A common) asks the same of every pimdir frontend: queued writes worded as queued.

pimdir-queue-visibility (2026-08-27) kept the shared confirmations identical across backends. That holds for the output shapes: no queue row id or other pimdir detail reaches a shared output (backends, "Shared outputs carry no backend details"). Whether a write is done or waits for a sync is not such a detail: any backend that queues its writes has the same answer to give, and every other backend answers "done".

## What

- `EmailClient::queues_writes`, true when the storage backend queues its writes for a sync engine to apply (pimdir), false otherwise.
- The confirmations of `message send`, `message add`, the composers' save and send, `message delete` (to the trash or from it), `message move`, `message copy` and `flag add|set|remove` say "queued" on such a backend: *Message queued for sending*, *Queued the move of 2 message(s) to the trash*, *Flags queued for adding on 2 message(s): seen*.
- No JSON shape changes: a structured output keeps its fields, and the `message` prose of a `Message` confirmation carries the wording. A queue row id still never reaches a shared output.
