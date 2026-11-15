---
cairn: tasks
change: a-wrong-weekday-keeps-its-date
---

# Tasks

- [x] src/email/envelope.rs `parse_rfc2822_date` and `strip_weekday`, with their tests, moved from src/imap/backend.rs; src/gmail/backend.rs drops its strict `parse_rfc2822`.
- [x] Test on the scripted Gmail server: `a_wrong_weekday_keeps_its_date`, failing on the old code (`date` `None`).
- [x] Fold into cairn/spec/commands.md; CHANGELOG; the log entry.
