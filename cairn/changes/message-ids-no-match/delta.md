---
cairn: change
change: message-ids-no-match
---

# Delta

## ADDED Requirements

### Requirement: A command given message ids says how many it found
`flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL fail, exit code 1, when none of the given ids exists in the mailbox, changing nothing. When some exist they SHALL succeed and report how many: the flag commands as `count` beside `flags`, the others as they already do. `message delete` on a message already in the trash is not covered. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya flag add --flag flagged 99999999` runs
- THEN the command exits 1 with `No flag added: no id matched in the mailbox`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 0 and prints `{"flags":["flagged"],"count":1}`

### Requirement: IMAP flag counts
The IMAP adapter's `store_flags` SHALL return how many of the given UIDs exist in the mailbox. A `UID STORE` echoing as many messages as UIDs given counts them all; one echoing fewer, which a server may do for a message whose flags did not change, SHALL be followed by a `UID SEARCH UID <set>` whose matches are the count.
