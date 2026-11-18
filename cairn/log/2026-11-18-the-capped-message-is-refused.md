---
cairn: log
change: the-capped-message-is-refused
landed: 2026-11-18
---

# The capped message is refused

Finding HIM-4 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/shared/message/part.rs: `measure` refuses an entity mime-rs marked `PartLimit` or `NestingLimit`.
- Test `a_message_past_the_parser_cap_is_refused`, failing before the fix.
- CHANGELOG (Fixed).
- io-pimdir's invitation and mark walk reads the same trees; the mark it can refuse on exists in mime-rs 0.2.0.

## Capabilities moved

- [commands](../spec/commands.md): *A message too complex is refused whole* and the error code table.
