---
cairn: log
change: single-reads-print-raw
landed: 2026-11-18
---

# Single reads print raw

The owner's ruling of 2026-11-18 on the single-resource protocol reads (schema review A6, its remainder), extending D18.

## What landed

- Gmail: `profile get`, `drafts get`, `messages get`, `threads get`, the settings reads, and the settings sets through `settings::raw_put`, print Gmail's payload.
- `msgraph messages get` prints Graph's resource; `jmap vacation-response get` sends a raw `VacationResponse/get`.
- Tests asserting an unmodelled key survives, one per read (listed in the change's tasks).
- CHANGELOG (Changed, breaking), MIGRATION (v2 to v3).

## Capabilities moved

- [commands](../spec/commands.md): *Data commands serialize their data*, *Gmail header selection applies to every format*, *Protocol listings print the provider's payload*.
