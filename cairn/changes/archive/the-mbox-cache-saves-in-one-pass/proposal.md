---
cairn: change
id: the-mbox-cache-saves-in-one-pass
status: landed
created: 2026-11-18
---

# The mbox cache saves in one pass

## Why

Finding HIM-16 of the family's cli-family-analysis-2026-11-15 plan: saving the mbox cache ran a linear `contains` per cached envelope, quadratic in the mailbox (0.47 s, 1.34 s and 4.58 s per `flag add` at 10,000, 20,000 and 40,000 messages, debug build). The id-list deduplication and the Maildir per-id lookups named beside it are left to another change.

## What

The index ids in a `HashSet`.
