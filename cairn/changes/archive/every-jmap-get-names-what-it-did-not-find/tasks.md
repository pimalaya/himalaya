---
cairn: tasks
change: every-jmap-get-names-what-it-did-not-find
---

# Tasks

- [x] src/error.rs: the codes `thread-not-found`, `identity-not-found` and `submission-not-found`.
- [x] src/jmap/backend.rs: `not_found_errors`, the item errors of a get's `notFound`, which `get_errors` now opens with.
- [x] src/jmap/thread/get.rs, src/jmap/identity/get.rs, src/jmap/submission/get.rs, query.rs, create.rs: `errors` on the outputs, printed last in text, `partial_when` after the result.
- [x] Tests over the scripted JMAP server: `a_thread_get_reports_each_id_not_found`, `an_identity_get_reports_each_id_not_found` and `a_submission_get_reports_each_id_not_found` (an entry coded as found, a partial outcome); each fails on the old commands, which returned success.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
