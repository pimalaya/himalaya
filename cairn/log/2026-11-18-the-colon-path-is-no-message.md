---
cairn: log
change: the-colon-path-is-no-message
landed: 2026-11-18
---

# The colon path is no message

Finding HIM-5 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/shared/message/arg.rs: `opens_on_a_field` holds the name to header characters and refuses a drive; `resolve` tests the path before decoding escapes.
- Tests `a_missing_path_holding_a_colon_is_not_a_message` and `a_path_holding_a_backslash_is_read`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [commands](../spec/commands.md): *Raw message input is shared*.
