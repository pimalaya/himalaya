---
cairn: log
change: a-seen-read-fails-unstaged
landed: 2026-11-18
---

# A seen read fails unstaged

Finding HIM-17 of the family's cli-family-analysis-2026-11-15 plan, its `--seen` point.

## What landed

- src/pimdir/backend.rs `get_message`: the staging failure fails the read.
- Test `a_read_whose_seen_flag_is_refused_fails`, failing on a04ff103.
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [backends](../spec/backends.md): *A read asked to mark the message seen fails unmarked* (added).
