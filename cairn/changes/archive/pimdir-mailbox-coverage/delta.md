---
cairn: change
change: pimdir-mailbox-coverage
---

# Delta

## ADDED Requirements

### Requirement: pimdir lists each mailbox with its coverage
`pimdir mailbox list` SHALL list the account's mailboxes sorted by id, each as `{"id", "name", "role", "total", "unread", "coverage", "round"}`: `role` as `mailbox list` shows it (`null` when none), `total` and `unread` counted in the store, `coverage` the collection's coverage (pimdir STORAGE §14.1, the narrowest of its sources') as `{"since", "until", "at"}`, and `round` a round a source has under way as `{"since", "until", "startedAt"}`. Bounds are RFC 3339 instants, `null` for an open end. `coverage` SHALL be `null` until every source of the collection closed a round on it, and on a store its owner has not reconciled; `round` SHALL be `null` when no round is under way. The table SHALL show the coverage and the round as their days (`since 2026-09-07`, `all`). The shared `mailbox list` SHALL carry none of it.

#### Scenario: A mailbox synced from a date
- GIVEN a pimdir account whose `imap` source closed a round on `imap/INBOX` over the mail since 2026-09-07
- WHEN `himalaya --json pimdir mailbox list` runs
- THEN `imap/INBOX` carries `"coverage":{"since":"2026-09-07T00:00:00Z","until":null,"at":…}`, and a mailbox no round closed on carries `"coverage":null`

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner
An envelope SHALL be built from the item's typed mail summary and the addresses joined to it (pimdir STORAGE Annex A.1, A.6): every `From` and `To` address with its display name, the `In-Reply-To` list, the date, the size and the attachment mark as stored, with no body read. `has-attachment` SHALL be `true` or `false` as the store marks it, and `null` on a row that holds no mark. A listing SHALL take the store's own order, newest first, the mail sort key being the date.
