---
cairn: delta
change: a-move-onto-its-own-mailbox-changes-nothing
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A command given message ids says how many it found
`flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL require ids, a repeated id being acted on and counted once. An id the mailbox does not hold SHALL be an `errors` entry coded `message-not-found`, the others acted on; when none exists, nothing SHALL change and the command SHALL print its result with `count` 0 and one entry per id, exiting 3. They SHALL report how many were acted on as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `raw` spelled as the backend writes it (`$flagged` on JMAP), `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `pimdir flag add|set|remove` SHALL fail, exit code 1, with the code `message-not-found` when none of the given ids exists, changing nothing. `message delete` in the trash SHALL count the ids the trash holds and remove those alone. On a backend whose ids are account-wide (JMAP, Gmail, Microsoft Graph), an id SHALL count as found only when its message is in the given mailbox. A move whose source and destination are one mailbox SHALL change nothing, on every backend, counting the ids the mailbox holds: IMAP sends no `MOVE` nor `COPY`, the UIDs kept, mbox leaves the file as it was, the raw `mbox message move` printing the ids given, and pimdir stages nothing. On Maildir and mbox every id SHALL be looked up before the first write, an unknown one an entry, a removal from the trash included. The protocol commands of the local stores taking ids (`maildir` and `mbox` `message copy|move`, `maildir` and `mbox` `flag add|set|remove`) SHALL require them too and look every id up before the first write, an unknown one refusing the whole set and nothing changed: naming every unknown id under `message-not-found`; the flag commands SHALL print each id once. A `maildir message move` whose source and target are one Maildir, without `--subdir`, SHALL change nothing, the ids kept. `maildir message copy|move` and `mbox message copy|move` SHALL print the ids the messages have in the target, each message once. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya --json flag add --flag flagged 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":0,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

#### Scenario: An mbox move onto its own file
- GIVEN an mbox mailbox `A` holding two messages
- WHEN `himalaya --json message move -f A -t A <id>` runs, then `himalaya --json mbox message move -m A -t A <id>`
- THEN the first prints `count` 1, the second `{"ids":["<id>"]}`, both exit 0, and the file is byte for byte what it was

## REMOVED Requirements

None.
