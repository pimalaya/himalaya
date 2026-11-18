---
cairn: log
change: a-pimdir-write-is-one-batch
landed: 2026-11-18
---

# A pimdir write is one batch

Finding HIM-9 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/pimdir/client.rs `enqueue_all`, src/pimdir/backend.rs deletes, copies, moves and flags through it.
- Test `a_refile_refused_on_one_id_stages_none`, failing on e440b4e4.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *pimdir is a reader and a producer, never the owner*.
