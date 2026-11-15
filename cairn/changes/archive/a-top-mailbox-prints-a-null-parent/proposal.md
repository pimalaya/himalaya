---
cairn: change
id: a-top-mailbox-prints-a-null-parent
status: landed
created: 2026-11-15
---

# A mailbox at the top prints a null parent

## Why

Task H28 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (reports/himalaya/part2-pimdir.md, F1): `himalaya --json pimdir mailbox create Pimalaya-Test-himalaya-1` printed `{"row":89,"name":"Pimalaya-Test-himalaya-1","source":"google/gmail"}`, leaving `parent` out, where backends.md prints `"parent": collection | null` and "Absent is null" holds for every key. `PimdirMailboxCreateOutput::parent` carried `skip_serializing_if`.

## What

`parent` prints `null` at the top, a key the schema now requires.
