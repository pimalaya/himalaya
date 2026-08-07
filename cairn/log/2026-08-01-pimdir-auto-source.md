---
cairn: log
change: pimdir-auto-source
landed: 2026-08-01
---

# pimdir backend auto-sources its writes

The pimdir backend now auto-detects the replica source for its writes when
`pimdir.source` is unset: it probes `PimdirStore::distinct_sources()` and, for a
store synced as a single source (the local-sync case), writes as that one source —
no configuration, no wrong-by-default `local` guard failure. It falls back to
`local` only when the store has no or several sources.

Verified: with no `pimdir.source` configured, a `\Seen` edited in Himalaya against
a one-side neverest store propagates to the remote on the next sync (the neverest
report shows the push). Build/test/fmt clean.

Spec updated: `backends` (ADDED: pimdir writes auto-source).
