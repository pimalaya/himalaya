---
cairn: change
id: a-jmap-get-names-what-it-did-not-find
status: landed
created: 2026-11-15
---

# A JMAP get names what it did not find

## Why

Task H30 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (reports/himalaya/jmap-fastmail.md, O1): `himalaya --json jmap email get Mbogus` exited 0 with `{"emails":[]}`, and `jmap mailbox get Pbogus` with `{"mailboxes":[]}`, while the server answered `"notFound":["Mbogus"]`; the id reached a WARN log line alone, so a script could not tell an unknown id from an empty answer. An object that did not deserialize was dropped without a word.

## What

`jmap email get|query` and `jmap mailbox get|query` print `errors` beside their collection, the family's partial outcome: one item error per id the server did not find (`message-not-found`, `mailbox-not-found`) and per object that did not deserialize (`message-unreadable`, `mailbox-unreadable`), printed last in text, and a non-empty `errors` exits 3. A query lists what its chained get answered, so an email gone between the two shows there too. The other protocol commands keep printing no `errors`; `jmap thread|identity|submission get` still drop their `notFound` (left open).
