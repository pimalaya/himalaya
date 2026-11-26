---
cairn: tasks
change: a-prefix-or-phrase-is-never-dropped
---

# Tasks

- [x] Owner: fix before the 3.0.0 tag, or ship and fix in 3.0.1. (2026-11-26: before the tag; IMAP and JMAP answer a prefix by substring, the open point)
- [x] src/unified/email/search/query.rs: `prefix` and `phrase` on the text leaves of the lowered tree (`SearchEmailsFilterText`, src/unified/email/search/filter/query.rs).
- [x] Translations: src/msgraph/search.rs (phrase quoted whole, prefix as `\"word*\"`), the Gmail `q` (prefix refused `search-unsupported`), IMAP and JMAP (phrase as one string, prefix answered by substring).
- [x] Tests failing first: `subject:repo*` and `subject:"report weekly"` through the Gmail and Graph translators, and the lowering.
- [x] Spec search.md (*Per-backend translation*, the text capability details), CHANGELOG `[3.0.0]`, log entry; re-run the search rows of gmail.md and msgraph.md (2026-11-26, the release build, and imap-smtp-fastmail.md P3).
