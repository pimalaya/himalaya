---
cairn: change
id: the-id-lists-scale
status: landed
created: 2026-11-19
---

# The id lists scale

## Why

Finding HIM-16 of the family's cli-family-analysis-2026-11-15 plan, what the-mbox-cache-saves-in-one-pass (2026-11-18) left: the ids of a unified write were deduplicated with a linear `contains` per id, and its errors ordered with a linear search per comparison (0.73 s and 2.42 s for 8,000 and 16,000 ids, 41.5 s at 64,000); a Maildir write located each id with its own read of the folder, then again in the write.

## What

The deduplication through a `HashSet`, the order through a map built once; the Maildir lookup of every id from one read (io-maildir 0.5's `locate_many`). No behaviour changes; the existing tests lock the order and the counts.
