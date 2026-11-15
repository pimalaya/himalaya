---
cairn: change
id: every-jmap-get-names-what-it-did-not-find
status: landed
created: 2026-11-15
---

# Every JMAP get names what it did not find

## Why

Task H34 of the family's cli-alignment-2026-11-15 plan, left open by a-jmap-get-names-what-it-did-not-find (task H30): `jmap thread get`, `jmap identity get` and `jmap submission get` still sent the server's `notFound` to a WARN log line alone, printing an empty list and exiting 0 for an unknown id, where `jmap email get` and `jmap mailbox get` now print an `errors` entry and exit 3. `jmap submission query` dropped the `notFound` of its chained get without a word.

## What

- `jmap thread get`, `jmap identity get` (given ids) and `jmap submission get|query` print `errors` beside their collection: one item error per id the server did not find, printed last in text, a non-empty `errors` exiting 3. A submission query lists what its chained get answered, as `jmap email query` does. `jmap submission create`, printing the same shape, prints `errors` as `[]`. io-jmap returns no unparsed object for these three gets, so no `*-unreadable` entry exists for them.
- The codes: the mail domain gives the nouns `message-` and `mailbox-` (pimalaya-cli cairn/spec/error.md, *A code names the domain's noun*), and neither names a thread, an identity or a submission, objects of their own in RFC 8621 with ids of their own. Three codes join the vocabulary, each `<noun>-not-found` after the RFC 8621 object it names: `thread-not-found`, `identity-not-found`, `submission-not-found`. `message-not-found` was not taken: a script reading it would take the id for an email's. No other binary of the family has these objects, so no twin needs them.
