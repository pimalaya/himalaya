---
cairn: change
id: pimdir-auto-source
status: landed
created: 2026-08-01
---

# pimdir backend auto-sources its writes

## Why

The pimdir backend attributes staged writes to a replica source (`pimdir.source`),
which for propagation must match the source the sync drives. Requiring the user to
know and set that name was a wart — and wrong-by-default (`local`), which failed
loudly on a store synced as `left`.

## What

When `pimdir.source` is unset, the backend **auto-detects** it: it reads the
store's `distinct_sources()` and, for a store synced as a single source (the
local-sync case), writes as that one source — no configuration. It falls back to
`local` only when the store has no or several sources.

Verified: with no `pimdir.source`, a flag edited in Himalaya against a one-side
neverest store propagates to the remote on the next sync.

## Scope / non-goals

- Multi-source stores still need an explicit `pimdir.source` (or fall back to
  `local`); the clean case is the single-source local cache.
