---
cairn: log
change: a-refused-write-leaves-no-body
landed: 2026-10-30
---

# A refused pimdir write leaves no body behind

From the pimdir stable-release Phase A, after io-pimdir's waves 3a to 3c of 2026-10-30 (its logs what-the-app-ports-found, a-query-only-index, store-audit-2026-10-30). Himalaya built against them unchanged: it streams no blob (`PimdirBlobs::reader`), drains nothing (`PimdirDrainReport`), and never edits a body it read, so no queued `update` and no `if_match` to state.

- `add_message` and `send_message` staged the body first (`stage_body`), then ran the gate and the duplicate check while enqueueing, so a write refused there left its body unpinned in the blob tree until a collector ran. Both now go through `PimdirProducer::enqueue_with_body_checked`: the gate and, for an add, the duplicate check on the bytes in hand, then the body written and the row appended in one transaction, the partial supports warned from its answer. Tests `a_refused_add_leaves_no_body_behind` (an `item-duplicate` add) and `a_refused_send_leaves_no_body_behind` (a send asking for a copy its performer does not declare, `unsupported`), both failing on the old code with "a refused add left its body behind" and "a refused send left its body behind". The collector tests of a-staged-body-is-written-under-the-lock pass unchanged.
- `store-foreign` (a database at the root declaring tables at `user_version` 0, io-pimdir `PimdirError::Foreign`) is relayed. Test `a_database_another_program_wrote_is_refused_as_foreign`, failing on the old code with `left: None`. `collection-queued`, `collection-role` and `cutoff-invalid` come from owner calls Himalaya never makes, and are not relayed.

Capabilities moved: **backends** (A pimdir body is written under the staging lock, A pimdir store failure carries its code, the added-message paragraph), **commands** (the JSON error code table).
