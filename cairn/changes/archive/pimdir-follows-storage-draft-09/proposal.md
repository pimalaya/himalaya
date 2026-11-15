---
cairn: change
id: pimdir-follows-storage-draft-09
status: landed
created: 2026-11-06
---

# The pimdir backend follows storage-draft-09 and search-draft-09

## Why

io-pimdir 5c18a02 (pushed, master, 0.9.0) follows pimdir storage-draft-09, sync-draft-08 and search-draft-09 (io-pimdir cairn/log/2026-11-05-the-crate-follows-storage-draft-09.md). Himalaya pins 0c70144 (storage-draft-08, 0.8), so it cannot read a store a draft-09 neverest writes, nor state a query's zone (change pimdir-search-reads-local-days, which this move carries).

## What

- `io-pimdir = "0.9"`, the `[patch.crates-io]` git rev moved to 5c18a02.
- io-pimdir 0.9 takes ical-rs 0.7, vcard-rs 0.6 and mime-rs 0.1 from crates.io. Himalaya's mime-rs moves from git d7ea032 to 0.1 (no public API change between them), and the ical-rs patch to 99b9273 goes, nothing in the graph wanting it any more: one mime-rs, one ical-rs.
- What io-pimdir changed that Himalaya reads: `PimdirSearchClock` and `PimdirSearchOptions` are no longer `Copy` (Himalaya passes them by reference), `PimdirSearchClock::zone` (the other change), `PimdirIndex::floating_zone` and `PimdirLoadSelection::Owing` (not read: mail has no floating time, and Himalaya never loads for a sync), flags as bits of the row (internal to the store).
- A draft-08 store is refused as `store-version`, never migrated. The message already says to upgrade Himalaya when the store's sync engine is newer, else to reinstall the store as neverest's MIGRATION.md says; it names no draft, so it reads right. The tests stamping another draft move to the draft just left (`storage-draft-08`, `search-draft-08`) and the next one (`storage-draft-10`).
