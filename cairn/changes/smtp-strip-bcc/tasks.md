---
cairn: tasks
change: smtp-strip-bcc
---

# Tasks

- [x] src/smtp/backend.rs: `strip_bcc`, applied to the bytes handed to `DATA` after the envelope is derived.
- [x] Tests: field removed and body untouched, folded continuation lines and any case, obsolete spelling with whitespace before the colon, a message without `Bcc:` byte-identical.
- [x] Manual check over SMTP: a blind recipient receives the message and the received copy carries no `Bcc:` field.
- [x] CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/backends.md](../../spec/backends.md); write the log entry.
