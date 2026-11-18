---
cairn: change
id: local-listings-hold-a-few-messages
status: landed
created: 2026-11-18
---

# Local listings hold a few messages

## Why

Finding HIM-8 of the family's cli-family-analysis-2026-11-15 plan: a Maildir listing read every file and kept them all before building envelopes (69 MB peak to list 5 envelopes of a 41 MB Maildir), and an mbox listing kept the bytes of every message it read, every body under any filter.

## What

- Maildir reads its files 64 at a time, each envelope built and matched before the next files.
- mbox hands each message's bytes to the filter and drops them before the next.
- Kept simple: the envelopes are still all built before sorting.
