---
cairn: delta
change: every-jmap-get-names-what-it-did-not-find
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A command acting on several items lists what failed
`envelope list`, `envelope search`, `pimdir search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1, and so SHALL a failure of the exchange itself met part-way (the connection, a batch answer that does not parse), only the server's refusal of an item carrying on. A listing reading the whole mailbox before cutting its page (Maildir, mbox, a JMAP search rechecked client-side) SHALL print its errors on its first page only. The commands acting on one item (`message read`, `message add`, `message send`, the composers, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`, but the JMAP gets and the queries chaining one: `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` SHALL print one entry per id the server did not find, coded `message-not-found`, `mailbox-not-found`, `thread-not-found`, `identity-not-found` or `submission-not-found`, and per email or mailbox that did not deserialize, coded `message-unreadable` or `mailbox-unreadable`, after the ones they list. `jmap submission create`, printing the shape of a get, SHALL print `errors` as `[]`.

#### Scenario: A JMAP get of an unknown id
- GIVEN a JMAP account whose server answers `Email/get` of `E1` and `Mbogus` with `E1` and `"notFound":["Mbogus"]`
- WHEN `himalaya --json jmap email get E1 Mbogus` runs
- THEN it prints `E1` under `emails` and `"errors":[{"id":"Mbogus","code":"message-not-found",…}]`, and the exit status is 3

#### Scenario: A JMAP thread get of an unknown id
- GIVEN a JMAP account whose server answers `Thread/get` of `T1` and `Tbogus` with `T1` and `"notFound":["Tbogus"]`
- WHEN `himalaya --json jmap thread get T1 Tbogus` runs
- THEN it prints `T1` under `threads` and `"errors":[{"id":"Tbogus","code":"thread-not-found",…}]`, and the exit status is 3

### Requirement: A JSON error carries a stable code
The code table gains three rows: `thread-not-found`, an `errors` entry of `jmap thread get` for an id the JMAP server did not find; `identity-not-found`, one of `jmap identity get`; `submission-not-found`, one of `jmap submission get` and `query`.

## REMOVED Requirements

None.
