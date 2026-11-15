---
cairn: change
id: pimdir-mailbox-coverage
status: landed
created: 2026-10-07
---

# pimdir lists each mailbox with its coverage

## Why

io-pimdir `35a1c3f` (pimdir scoped-mail-sync) lets a sync engine list a mailbox within a scope, its mail since a date, in pages, and drops the probe tier and its `probes` table. A store a new owner reconciled has no `probes` table, so Himalaya has to move to that io-pimdir to keep reading the stores neverest writes. A reader such as MOA then needs to say "mail since …" and to know a search is not exhaustive below that date, without reading the store itself: Himalaya is how it reads one.

## What

- io-pimdir pinned to `35a1c3f`; nothing read the probe tier beyond a test.
- `pimdir mailbox list`: each mailbox of the account with its counts, its coverage (`{since, until, at}`, `null` until every source closed a round on it) and the round under way (`{since, until, startedAt}`, `null` when none is). The shared `mailbox list` stays free of pimdir details (shared-output-free-of-pimdir).
- The envelope's `has-attachment` is the stored attachment mark (pimdir Annex A.1): `true`, `false`, or `null` on a row that holds none. It already read the summary; the requirement now says so, and a test pins the three values.
