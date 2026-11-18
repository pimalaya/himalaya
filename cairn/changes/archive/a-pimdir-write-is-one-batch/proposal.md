---
cairn: change
id: a-pimdir-write-is-one-batch
status: landed
created: 2026-11-18
---

# A pimdir write is one batch

## Why

Finding HIM-9 of the family's cli-family-analysis-2026-11-15 plan: a pimdir `message delete`, `copy` or `move` of several ids staged one queue transaction per id, where flag writes use one `enqueue_batch`. The gate refusing the nth id left the first n-1 queued behind a whole failure (exit 1, no receipt), and a rerun staged them again. The spec says a pimdir write, one queue transaction, stays a whole failure.

## What

One `PimdirClient::enqueue_all` appends a command's rows in one batch, naming the id the gate refuses; flags, deletes, copies and moves go through it.
