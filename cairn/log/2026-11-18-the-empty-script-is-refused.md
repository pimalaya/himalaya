---
cairn: log
change: the-empty-script-is-refused
landed: 2026-11-18
---

# The empty script is refused

Finding HIM-19 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/sieve/activate.rs: `NAME` refuses an empty value.
- Test `an_empty_name_is_refused`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [commands](../spec/commands.md): *ManageSieve commands*.
