---
cairn: change
change: pimdir-auto-source
---

# Delta

## ADDED Requirements

### Requirement: pimdir writes auto-source
When `pimdir.source` is unset, the pimdir backend SHALL attribute its writes to the
store's single synced source (via `distinct_sources`) when there is exactly one —
the local-sync case — so a staged edit propagates without configuration, falling
back to `local` only when the store has no or several sources.

## MODIFIED Requirements

## REMOVED Requirements
