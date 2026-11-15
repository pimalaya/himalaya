---
cairn: log
change: mbox-flag-set-needs-a-flag
landed: 2026-11-15
---

# mbox flag set needs a flag

Task H24 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (mbox.md, H-6): `mbox flag set` without `-f` wiped every flag.

## What landed

- src/mbox/flag/set.rs: `--flag` is required, as on `maildir flag set`.
- Test: tests/local_stores.rs `an_mbox_flag_set_without_a_flag_is_refused`, exit 0 and a cleared `Status` before.

## Capabilities moved

- [commands](../spec/commands.md): *The raw Maildir folder commands guard the store*, `mbox flag set` requiring `-f` too.
