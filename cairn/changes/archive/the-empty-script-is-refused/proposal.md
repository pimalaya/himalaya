---
cairn: change
id: the-empty-script-is-refused
status: landed
created: 2026-11-18
---

# The empty script is refused

## Why

Finding HIM-19 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. `sieve activate ""` sent `SETACTIVE ""`, which RFC 5804 defines as a deactivation, and printed "Sieve script successfully activated": with an unset shell variable, filtering, vacation and redirects stopped behind a success message. `sieve deactivate` is the command that deactivates.

## What

- The `NAME` of `sieve activate` (src/sieve/activate.rs) goes through clap's `NonEmptyStringValueParser`, so an empty one is a usage error before connecting.
