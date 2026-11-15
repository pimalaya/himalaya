---
cairn: log
change: a-wrong-weekday-keeps-its-date
landed: 2026-11-15
---

# A wrong weekday keeps its date

Task H21 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (gmail-google.md, H-2): a Gmail envelope dropped a `Date:` naming the wrong weekday.

## What landed

- src/email/envelope.rs: `parse_rfc2822_date`, retrying without the day-of-week token chrono checks, moved from src/imap/backend.rs with its four tests; IMAP and Gmail both read through it.
- src/gmail/backend.rs: `envelope_from` reads the metadata `Date` with it; the strict `parse_rfc2822` is gone.
- Test: `a_wrong_weekday_keeps_its_date`, a scripted Gmail listing whose message is dated `Mon, 15 Nov 2026 11:00:00 +0000`, now `2026-11-15T11:00:00+00:00`, `None` before.

## Capabilities moved

- [commands](../spec/commands.md): *A wrong weekday keeps its date* added.
