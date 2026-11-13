---
cairn: change
id: pimdir-follows-draft-11
status: landed
created: 2026-11-13
---

# The pimdir backend follows storage-draft-11, sync-draft-11 and search-draft-11

## Why

pimdir landed storage-draft-11, sync-draft-11 and search-draft-11 on 2026-11-13 (pimdir d8800fb, the fourteen lines draft-10 parked for the owner), and io-pimdir follows them in cd33f0e, its version still 0.11.0 and not released. Drafts are exact, so a Himalaya on io-pimdir 0.11.0 from crates.io refuses every store a draft-11 neverest writes.

## What

- The pin: io-pimdir at cd33f0e through `[patch.crates-io]`, the dependency line staying `0.11`. One site stopped compiling, a test building `PimdirSearchReads` by a struct literal, which gained `mark`.
- The draft names: a draft-10 store or index is the earlier one, refused as `store-version` or `index-version`; the tests stamping a later draft take draft-12.
- The coverage: a `bodiless` message leaves the answer incomplete, and is named in the `Incomplete:` line, only while a term reads a message's body, `thread:` among them, or under `has:attachment` (SEARCH §8.2), as `missing` and `unparseable` already were. Himalaya reads the condition from io-pimdir (`PimdirSearchLocal::reads`, `PimdirSearchReads::counts_bodiless`). `complete` stays io-pimdir's `is_complete`.
- A move over a conflict: `message move`, and `message delete` when it moves to the trash, queue a `move`, which states no resolution. The owner now parks it over a message a sync left in conflict (SYNC §7), and the reader's overlay folds nothing for it (STORAGE §15.4). No code of Himalaya's moves: the message stays listed, and the row reads `parked` with why in `pimdir queue list` and `pimdir queue show`, as a delete over a conflict does since draft-10.
- A group standing as a conjunct of the whole query is read through, and `subject:a@b` is a text term: the grammar's, Himalaya holding no check of its own.

## Out of scope

A view of conflicts and a `queue retry`, without which a move or a delete parked over a conflict can only be cancelled. The suites of tests/ and an end-to-end run on a scratch store a draft-11 neverest synced.
