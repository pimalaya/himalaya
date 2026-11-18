---
cairn: delta
change: a-sent-message-is-a-result
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL NOT fail the command: the receipt SHALL say `sent: true` and `saved: false`, and list the failure in its `errors`, `id` `null`, coded by its cause, its `error` stating that the message was sent, and the command SHALL exit 3, so a caller never sends it twice. The receipts of `message send`, the composers and `message add` SHALL print `errors`, `[]` when nothing failed after the send. A backend filing every sent message itself (Gmail, under `SENT`) SHALL file the saved copy by labelling the message it sent with the mailbox and the flags, rather than add a second message, and SHALL report that message's id; pimdir files it through its queue. The save of a send SHALL be checked against `submit.copy`, the sending transport's, and a save alone against `message.add`. Graph files every message it sends in Sent Items itself and declares `submit.copy` `partial` with that detail: on a send, `--save` and `message.send.save-copy` SHALL be ignored, nothing added, and the receipt SHALL say `saved: true` with a `null` id. `--no-save` SHALL be refused there before connecting, coded `capability-missing` and naming the flag, Graph keeping the copy whatever is asked; a save without a send stays refused, `message.add` being `none` (backends: "Append gap").

#### Scenario: A copy the store refuses after the send
- GIVEN a Maildir account sending over SMTP
- WHEN `himalaya --json message send --save Nowhere <message>` runs and the server accepts the message
- THEN it prints `{"id":null,"saved":false,"sent":true,"queued":false,"errors":[{"id":null,"code":"mailbox-not-found","error":"Message sent, but saving a copy to Nowhere failed",…}]}` and exits 3

#### Scenario: A Graph send keeping no copy
- GIVEN an account whose storage backend is Graph
- WHEN `himalaya --json message send --no-save <message>` runs, or `message compose --send --no-save`
- THEN it fails coded `capability-missing`, naming `--no-save`, before the token is asked for

### Requirement: A sent reply or forward flags its source
`message reply --send` SHALL add the `answered` flag (RFC 3501 §2.3.2) to the source message once the send has succeeded, where the backend serves `flags.answered`; on one declaring it `none` (Gmail, Graph) the flag SHALL be skipped and the command succeed. A flag failing after a successful send SHALL NOT fail the command: the one receipt SHALL list it in its `errors`, `id` the source message, its `error` stating that the message was sent, and the command SHALL exit 3, printing nothing after the receipt. `message forward --send` SHALL add the `forwarded` flag (RFC 5788) to the source the same way, best-effort: a backend refusing the keyword SHALL be logged as a warning and SHALL NOT fail the command. A reply or forward only printed or saved SHALL leave the source untouched.

### Requirement: A command acting on several items lists what failed
`envelope list`, `envelope search`, `pimdir search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1, and so SHALL a failure of the exchange itself met part-way (the connection, a batch answer that does not parse), only the server's refusal of an item carrying on. A listing reading the whole mailbox before cutting its page (Maildir, mbox, a JMAP search rechecked client-side) SHALL print its errors on its first page only. The commands acting on one item (`message read`, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`, but `message add`, `message send` and the composers, whose `errors` lists what failed after the message went out ("Saving a sent message follows the send"), but the JMAP gets and the queries chaining one: `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` SHALL print one entry per id the server did not find, coded `message-not-found`, `mailbox-not-found`, `thread-not-found`, `identity-not-found` or `submission-not-found`, after the ones they list, the objects listed printing as the server sent them ("Protocol listings print the provider's payload"). `jmap submission create`, printing the shape of a get, SHALL print `errors` as `[]`. The JMAP sets acting on several objects, `jmap email delete|update|copy`, `jmap mailbox destroy`, `jmap identity delete` and `jmap submission cancel`, SHALL print `{ids, errors}`: the ids of the objects the server applied the write to (for a copy, the ids of the copies), then one item error per object it refused, keyed by the id given and coded by its SetError type, a `notFound` `message-not-found`, `mailbox-not-found`, `identity-not-found` or `submission-not-found`.

#### Scenario: A listing with one unreadable message
- GIVEN a Maildir holding three messages, one of whose files cannot be read
- WHEN `himalaya --json envelope list` runs
- THEN it prints the two other envelopes and `"errors":[{"id":"<its id>","code":"message-unreadable",…}]`, and the exit status is 3

#### Scenario: A file holding no header field
- GIVEN a Maildir holding one message and an empty file, or an mbox holding one message and one whose lines hold no header field
- WHEN `himalaya --json envelope list` runs
- THEN it prints the message's envelope alone and `"errors":[{"id":"<the other's id>","code":"message-unreadable",…}]`, and the exit status is 3

#### Scenario: A JMAP get of an unknown id
- GIVEN a JMAP account whose server answers `Email/get` of `E1` and `Mbogus` with `E1` and `"notFound":["Mbogus"]`
- WHEN `himalaya --json jmap email get E1 Mbogus` runs
- THEN it prints `E1` under `emails` and `"errors":[{"id":"Mbogus","code":"message-not-found",…}]`, and the exit status is 3

#### Scenario: A JMAP thread get of an unknown id
- GIVEN a JMAP account whose server answers `Thread/get` of `T1` and `Tbogus` with `T1` and `"notFound":["Tbogus"]`
- WHEN `himalaya --json jmap thread get T1 Tbogus` runs
- THEN it prints `T1` under `threads` and `"errors":[{"id":"Tbogus","code":"thread-not-found",…}]`, and the exit status is 3

#### Scenario: A JMAP destroy applied in part
- GIVEN a JMAP account whose server answers `Email/set` destroying `E1` and `E2` with `"notDestroyed":{"Egone":{"type":"notFound"}}`
- WHEN `himalaya --json jmap email delete E1 E2 Egone` runs
- THEN it prints `{"ids":["E1","E2"],"errors":[{"id":"Egone","code":"message-not-found",…}]}` and the exit status is 3

