---
cairn: tasks
change: a-jmap-get-names-what-it-did-not-find
---

# Tasks

- [x] src/jmap/backend.rs: `get_errors`, the item errors of a get's `notFound` and unparsed objects.
- [x] src/jmap/email/get.rs, query.rs, src/jmap/mailbox/get.rs, query.rs: `errors` on the outputs, printed last in text, `partial_when` after the result.
- [x] Tests over the scripted JMAP server: `an_email_get_reports_each_id_not_found` and `a_mailbox_get_reports_each_id_not_found` (an entry coded as found, a partial outcome); both fail on the old commands, which returned success.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
