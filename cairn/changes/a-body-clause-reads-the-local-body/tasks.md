---
cairn: tasks
change: a-body-clause-reads-the-local-body
---

- [x] Test first, failing on the empty body: on a store holding local bodies, `body` finds the messages whose body holds the word, `not body` the others, and a bodiless message matches no `body` clause
- [x] `search_pushed` and `search_scanned` read the local body when the filter holds a `body` clause
- [x] Doc comment of `search_envelopes`
- [x] Fold into spec/search.md, log, CHANGELOG
