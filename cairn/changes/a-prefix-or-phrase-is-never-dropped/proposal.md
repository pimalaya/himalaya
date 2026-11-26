---
cairn: change
id: a-prefix-or-phrase-is-never-dropped
status: active
created: 2026-11-26
---

# A prefix or a phrase is never dropped

## Why

Found by the v3 provider reports of 2026-11-26 (cairn/spec/testing/gmail.md H1, msgraph.md M5) on the 3.0.0 release build. `envelope search` over Gmail and Microsoft Graph silently answers a query other than the one written: `subject:repo*` lists nothing where `subject:report` lists two messages (exit 0), and on Graph `subject:"report weekly"` matches "beta weekly report". `lower` (src/unified/email/search/query.rs:240) reads `PimdirQueryText::text` and never its `prefix` nor its `phrase`, so the server translations get a bare word; Graph's `value_kql` (src/msgraph/search.rs) then quotes each word apart. The help says a trailing `*` makes a word a prefix, and cairn/spec/search.md says a term a backend cannot answer is refused, "no term is dropped". Maildir, mbox and pimdir match through io-pimdir's `search::mail`, which reads both flags, and are not affected. A v3 query language bug: fixing it before the tag keeps 3.0.0 from shipping a silent mis-answer.

## What

- The lowered tree carries `prefix` and `phrase` on its text leaves.
- Each server translation serves them or refuses the term with `search-unsupported` before any request: Graph KQL takes a phrase as one quoted string and a prefix as `word*`; Gmail has no prefix search (refuse a prefix) and takes a phrase quoted; IMAP and JMAP match substrings, so a prefix is answered (`partial`, as their text is) and a phrase is sent as one string.
- The capability table states it (`envelope.search.text` and the text fields' details), the help names the backends refusing a prefix.

## Open

- Whether IMAP and JMAP should refuse a prefix rather than answer it by substring (they already over-match plain words the same way).
