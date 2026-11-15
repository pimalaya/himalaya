---
cairn: delta
change: a-queued-write-says-queued
---

## ADDED Requirements

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the confirmation of a shared write SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy` and `flag add|set|remove`. The JSON shapes SHALL NOT change, and no queue detail SHALL reach a shared output; the prose of a `message` confirmation carries the wording. Every other backend SHALL keep confirming the write as done.

#### Scenario: A send on a pimdir account
- GIVEN a pimdir account with `mailbox.alias.sent` set
- WHEN `message send` runs
- THEN it prints `Message queued for sending`, not that the message was sent

#### Scenario: A delete on a pimdir account
- GIVEN a pimdir account with a trash mailbox
- WHEN `message delete 3` runs on another mailbox
- THEN it prints `Queued the move of 1 message(s) to the trash`
