---
cairn: change
id: the-schema-leftovers
status: landed
created: 2026-11-18
---

# The schema leftovers

## Why

The three findings of the schema review of 2026-11-14 (reports/schema-review-2026-11-14.md of the family's cli-alignment-2026-11-15 plan) still open in Himalaya that break a consumer, after cairn/log/2026-11-15-the-schema-review-shapes.md:

- C3: instant keys read alike across the family (`queuedAt`, `startedAt`), but `pimdir mailbox list` printed a coverage's `at` and `pimdir search --threads` a thread's `first` and `last`.
- C6, its `warnings` half: `sieve check` and `sieve put` printed `warnings` as one string, where every receipt of the family prints a list.
- C12: `jmap email parse` printed `bodies` as strings, losing which blob each came from; the blobs not found or not parsable were only logged.

## What

- `pimdir mailbox list`: `coverage.at` becomes `coverage.closedAt`.
- `pimdir search --threads` and `pimdir thread`: `first` and `last` become `firstDate` and `lastDate`.
- `sieve check|put`: `warnings` is a list, one entry per line of the server's text, `[]` when it sent none.
- `jmap email parse`: `bodies` is `[{blobId, body}]`, beside `errors`, an item error per blob not found (`message-not-found`) or not parsable (`message-unreadable`), exit 3 when one is.
- Breaking: keys renamed and retyped. C6's other half, a `code` on the IMAP refusal, is additive and left out.
