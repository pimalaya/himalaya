---
cairn: log
change: a-top-mailbox-prints-a-null-parent
landed: 2026-11-15
---

# A mailbox at the top prints a null parent

Task H28 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (part2-pimdir.md, F1): `pimdir mailbox create --json` left `parent` out at the top.

## What landed

- src/pimdir/mailbox/create.rs: `parent` no longer skipped, `null` at the top, required by the schema; test `a_mailbox_at_the_top_prints_a_null_parent`, failing with the attribute back.

## Capabilities moved

- [backends](../spec/backends.md): *pimdir creates a mailbox by queueing an intent* says `parent` is never left out, with a scenario.
