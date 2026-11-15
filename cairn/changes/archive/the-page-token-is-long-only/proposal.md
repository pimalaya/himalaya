---
cairn: change
id: the-page-token-is-long-only
status: landed
created: 2026-11-15
---

# The page token is long only

## Why

Task H7 of the family's cli-alignment-2026-11-15 plan (finding F26, owner ruling D11: "drop it"). In Himalaya `-t` is `--to`, the recipient of `message compose` and `forward` and the target of `message copy` and `move`; only the paging flags of `pimdir attachment list` and `search` (src/pimdir/attachment/list.rs `PageArgs`) reused it for `--page-token`, which is long only everywhere else.

## What

`--page-token` loses its short letter on both commands, which share `PageArgs`. Breaking for a development build of v3 only, the commands being new in it: CHANGELOG and MIGRATION.md say so.
