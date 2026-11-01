---
cairn: change
id: a-search-says-what-it-covered
status: abandoned
created: 2026-10-18
---

# A pimdir search says what it did not cover

## Why

On the pimdir backend a search reads a partial replica, and says nothing about the part it could not read. Re-verified on 2026-10-18 in src/pimdir/backend.rs (`search_envelopes`): a body clause cannot match an item whose body is not local, and a date clause cannot reach mail older than what the store has listed, and both silently return fewer hits. `pimdir mailbox list` already shows each mailbox's coverage (pimdir-mailbox-coverage, 2026-10-07), but `envelope list` with a query does not use it, so an empty result reads as "no such message".

Borrowed from an external engine: a three-axis search coverage (what is local and indexed, the time range the store is complete for, whether a server was also asked), rolled up conservatively, with a single "complete" answer driving "older messages not downloaded" and "searching the server" signals.

## What

- `envelope list` on the pimdir backend computes the coverage of its query: the hits that could only match on headers because the body is absent, and whether the query's range reaches below the mailbox's coverage or into a round still filling.
- The table prints one line under the results when the search is not complete (*12 messages have no local body; mail before 2026-03-01 is not synced*); `--json` carries it as a `coverage` object; complete searches print nothing extra.
- The type and its roll-up come from io-pimdir (cairn/changes/search-feature, section "Coverage, three axes") so himalaya, the Android app and other clients say the same thing; until it lands, himalaya computes the two axes it can from the coverage rows it already reads.
- The remote axis stays `LocalOnly`: falling back to the account's server search is a separate change, if ever.

## Open questions

- Where the line goes when the output is a pipe: stderr, or only in `--json`.

## Closed

Abandoned on 2026-10-30, absorbed into pimdir-search with the plan the user approved that day: the coverage is carried by `pimdir search`, `count` and `tag` from io-pimdir's three-axis coverage (`bodiless`, `unindexed`, `behind`, the temporal bounds), a line under the table and a `coverage` object under `--json`. The shared `envelope list` keeps none, a shared output carrying no pimdir detail.
