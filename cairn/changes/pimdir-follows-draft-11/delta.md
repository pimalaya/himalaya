---
cairn: delta
change: pimdir-follows-draft-11
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir is a reader and a producer, never the owner (backends)

- The overlay folds nothing for a row the owner would park: a `remove` or a `move` staged on a message a sync left in conflict leaves it listed where it was, and a `move` brings it into no other mailbox.
- A `Move` states no resolution (pimdir SYNC §7): the owner parks it over a message a sync left in conflict, the move `message delete` makes to the trash included, where it left the conflict standing. Its row reads `parked` in `pimdir queue show` and in the `pimdir queue list` of the mailbox the message stays in. A `Copy` meets no conflict.

#### Scenario: A move over a message in conflict
- GIVEN a pimdir draft a sync left in conflict
- WHEN `himalaya message move` queues it to another mailbox, and the sync engine then drains the queue
- THEN the draft stays listed in its mailbox and the other mailbox never lists it, before the drain and after
- AND `pimdir queue list` on its mailbox and `pimdir queue show` on its row read `parked`, the error naming the diverging body

### Requirement: A pimdir search says what it did not cover (search)

- `bodiless` is the count of the scope whatever the query, and leaves the answer incomplete, and is said in the plain output, only while a term reads a mail body, `thread:` among them, or under `has:attachment`. It left every answer incomplete.
- What a term reads is io-pimdir's (`PimdirSearchLocal::reads`, `PimdirSearchReads::counts_bodiless`), restated nowhere.

#### Scenario: A body not local under a flag query
- GIVEN a pimdir mailbox whose round is closed, holding one message whose body is not local
- WHEN `himalaya --json pimdir search is:flagged` runs
- THEN the coverage carries `"bodiless":1` and `"complete":true`, and the plain output prints no `Incomplete:` line
- AND `subject:lunch`, `thread:a@x.org` and `has:attachment` over the same mailbox each carry `"complete":false`, the `Incomplete:` line naming the message with no local body

### Requirement: pimdir searches through the store's index (search)

- The query language is that of search-draft-11 §8.

### Requirement: A missing or outdated pimdir index is a coded error (search)

- The scenario An index of an earlier draft stamps `search-draft-10`.

## REMOVED Requirements

None.
