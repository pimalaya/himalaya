---
cairn: change
id: mbox-flag-set-needs-a-flag
status: landed
created: 2026-11-15
---

# mbox flag set needs a flag

## Why

Task H24 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (mbox.md, H-6): `mbox flag set -m D2 <id>` without `-f` cleared every flag of the message (its `Status` and `X-Status` gone) and exited 0, where `maildir flag set` refuses the same command line since the 2026-11-13 local stores campaign.

## What

`mbox flag set` requires at least one `-f`, clap refusing the command line (exit 2, `usage-invalid` under `--json`), as `maildir flag set` does; `mbox flag remove` drops flags by name.
