---
cairn: tasks
change: a-prefix-or-phrase-is-never-dropped
---

# Tasks

- [ ] Owner: fix before the 3.0.0 tag, or ship and fix in 3.0.1.
- [ ] src/unified/email/search/query.rs: `prefix` and `phrase` on the text leaves of the lowered tree.
- [ ] Translations: src/msgraph/search.rs (phrase quoted whole, prefix as `word*`), the Gmail `q` (prefix refused `search-unsupported`), IMAP and JMAP (phrase as one string, prefix answered by substring).
- [ ] Tests failing first: `subject:repo*` and `subject:"report weekly"` through the Gmail and Graph translators.
- [ ] Spec search.md (*Per-backend translation*, the text capability details), CHANGELOG `[3.0.0]` Fixed, log entry; re-run the search rows of gmail.md and msgraph.md.
