---
cairn: log
change: a-prefix-or-phrase-is-never-dropped
landed: 2026-11-26
---

# A prefix or a phrase is never dropped

The v3 provider reports of 2026-11-26 (gmail.md H1, msgraph.md M5, imap-smtp-fastmail.md P3) found `envelope search` lowering `repo*` and `"report weekly"` to bare words before the server translations. The owner ruled on 2026-11-26 to fix it before the 3.0.0 tag, IMAP and JMAP answering a prefix by substring.

## What landed

- src/unified/email/search/filter/query.rs: `SearchEmailsFilterText {text, phrase, prefix}` on every text and address leaf, rendered back as the query wrote it (`subject:repo*`, `subject:"report weekly"`); src/unified/email/search/query.rs `lower` keeps both marks.
- src/msgraph/search.rs `value_kql`: a phrase one quoted KQL string, a prefix its stem quoted with a trailing `*`.
- src/gmail/search.rs `text_q`: a prefix refused `search-unsupported` before any request, a phrase quoted whole as before.
- src/imap/backend.rs, src/jmap/backend.rs: the stem and the phrase sent as one string, unchanged in behaviour.
- src/unified/capability.rs: Gmail's text fields `partial` (a prefix refused), the IMAP and JMAP details naming the stem and the phrase, Cyrus and Fastmail finding the whole word alone and a phrase's words in any order; the `envelope search` help says how each backend takes a prefix and a phrase.

Tests failing against c3cdf686 (run from a `git archive` snapshot with the new tests): `a_prefix_and_a_phrase_survive_the_lowering` (`subject:repo` for `subject:repo*`), `a_prefix_and_a_phrase_keep_their_meaning` on Graph (`subject:\"repo\"`), `a_prefix_is_refused_and_a_phrase_quoted` on Gmail (the prefix translated). `a_prefix_is_its_stem_and_a_phrase_one_string` on IMAP and JMAP passes before and after: those translations already sent the stem, and pin it.

## Live recheck (release build, 2026-11-26, read-only)

- google (Gmail): `subject:integ*`, `integ*`, `from:goo*` exit 1 `search-unsupported`; `subject:"integration test"` 56 hits, `subject:"test integration"` none.
- microsoft (Graph): `subject:integ*` 24 hits as `subject:integration`, `subject:verif*` 7; `subject:"identity verification"` 4, `subject:"verification identity"` none; free `integ*` also matches `intégrées`, KQL folding accents.
- fastmail (IMAP, Cyrus): `subject:integ*` none, `subject:receipt*` the receipt (whole word only); `subject:"test integration"` matches `io-smtp integration test`, Cyrus taking a phrase's words in any order.

## Capabilities moved

- [search](../spec/search.md): *Per-backend translation* (the marks on the leaves, Gmail, Graph, IMAP and JMAP), *What a backend searches is declared*, *A field a backend does not answer is refused with a code* (Gmail a prefix), *The query's fields are listed on -h*; scenarios for a prefix on Gmail and a prefix and a phrase on Graph.
