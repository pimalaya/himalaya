---
cairn: delta
change: jmap-dates-read-every-offset
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Per-backend translation
(search) JMAP: on the chain of `and` from the root a day SHALL be bounded on the received date from below by its first instant in any offset (10:00 UTC the day before), then re-checked on the sent date.

## REMOVED Requirements

None.
