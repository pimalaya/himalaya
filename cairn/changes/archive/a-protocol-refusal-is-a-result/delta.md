---
cairn: delta
change: a-protocol-refusal-is-a-result
---

# Delta

## ADDED Requirements

### Requirement: A protocol refusal is a result
Inside the `gmail`, `msgraph`, `jmap`, `sieve` and `smtp` namespaces, a completion the server refuses SHALL print as the command's result and exit 3, as `imap` and the `raw` commands do (pimalaya-cli cairn/spec/output.md, *A protocol refusal is a result*); a command getting no completion (a network, TLS or decoding failure, a local error, a value refused before sending) SHALL print the failure object and exit 1. An HTTP 4xx or 5xx a `gmail`, `msgraph` or `jmap` command meets (the `request` commands included), a JMAP request refused whole (RFC 8620 §3.6.1), a JMAP method error (§3.6.2) and the SetError refusing the one write of a `jmap` command (§5.3) SHALL print `{refusal: {status, reason, message, code, retryAfter}}`: `status` the HTTP status, `null` for a refusal inside a 2xx; `reason` the provider's name for it in its spelling (Gmail's first reason, else its canonical status; Graph's `error.code`; the JMAP error type, the problem type's URN namespace stripped), `null` when it names none; `message` the provider's text, the type standing in when a JMAP error carries none; `code` the code the failure would carry (*A server's refusal codes the whole command*), `null` when none applies; `retryAfter` the seconds the server asked to wait, `null` when it asked none. A typed `sieve` command answered `NO` or `BYE` SHALL print the completion `{status, code, text}` as `sieve raw` prints it. `smtp send` refused SHALL print `{refusals: [{rcptTo, code, text}]}`, one reply per refusal in the order received (a refused `MAIL FROM` or `DATA` with `rcptTo` `null`, one per recipient when the server refused them all), the message reaching nobody. Each command's JSON Schema SHALL describe its own result and the refusal as two alternatives told apart by their keys.

#### Scenario: A Gmail id that does not exist
- GIVEN a Gmail account answering 404 `notFound`
- WHEN `himalaya --json gmail request get users/me/labels/nope` runs
- THEN it prints `{"refusal":{"status":404,"reason":"notFound","message":"Requested entity was not found.","code":null,"retryAfter":null}}` and exits 3

#### Scenario: A recipient the server refuses
- GIVEN an SMTP server answering `RCPT TO:<refused@example.org>` with `550 5.1.1 No such user`
- WHEN `himalaya --json smtp send -f t@example.org -t refused@example.org <message>` runs
- THEN it prints `{"refusals":[{"rcptTo":"<refused@example.org>","code":550,"text":"5.1.1 No such user"}]}` and exits 3

## MODIFIED Requirements

### Requirement: A server's refusal codes the whole command
A command failing whole because a server refused it SHALL carry the code of that refusal, read from the typed error of the protocol that answered it anywhere in the error chain: an IMAP completion naming `NOPERM` `permission-denied` and one naming `OVERQUOTA` `quota-exceeded` (RFC 5530 §3); a Gmail or Microsoft Graph answer as the same answer codes an item; a JMAP HTTP 429 `rate-limited` and 403 `permission-denied`, whichever request it answered, a method error `forbidden` `permission-denied`, and the SetError refusing the one write of a command by its type (`forbidden`, `overQuota`, `rateLimit`). The code SHALL be given at every exit: a one-shot run, a REPL line, an MCP call, and the row of an `account check`. Inside a protocol namespace the refusal SHALL be the command's result rather than a failure (*A protocol refusal is a result*), its `refusal.code` carrying that code. A command reading one message (`message read`, `attachment list`, `attachment download`, the source of `message reply` and `message forward`) whose id the backend does not hold SHALL fail coded `message-not-found` on every backend, a server's answer kept in `sources`.

#### Scenario: A mailbox the server forbids
- GIVEN an IMAP server answering `CREATE` with `NO [NOPERM]`
- WHEN `himalaya --json mailbox create Denied` runs, or the line `mailbox create Denied` in a `repl`, or the MCP call `mailbox_create`
- THEN each answers the failure object coded `permission-denied`

#### Scenario: A message the mailbox does not hold
- GIVEN a Maildir account
- WHEN `himalaya --json message read 999` runs
- THEN it exits 1 and prints the failure object coded `message-not-found`, naming `999` and the mailbox

### Requirement: The Gmail and Graph escape hatches are requests
When built with the `gmail` or `msgraph` feature, `gmail request <METHOD> <PATH> [JSON]` and `msgraph request <METHOD> <PATH> [JSON]` SHALL send one HTTP request under the account's bearer token, as Cardamum's `gpeople request` and `msgraph request` do: METHOD `get`, `post`, `patch` or `delete`, `put` too on Gmail, in any case, another refused before connecting; PATH joined onto `https://gmail.googleapis.com/gmail/v1/` or `https://graph.microsoft.com/v1.0/`, a leading `/` dropped, or a full URL taken as it is when it stays on that API's origin (`https://gmail.googleapis.com`, `https://graph.microsoft.com`), a URL off it refused before any request, the bearer token never leaving for another host; JSON the body of a `post`, `put` or `patch`, `null` when absent. They SHALL print `{response}`, the body the provider answered as it sent it, `null` for an empty one, its text form the body pretty-printed, nothing for an empty one. A refusal SHALL print as the typed commands' refusals do, under `refusal`, exit 3 (*A protocol refusal is a result*).

#### Scenario: A Graph patch
- GIVEN a Microsoft Graph account
- WHEN `himalaya --json msgraph request patch /me/mailFolders/F1 '{"displayName":"Work"}'` runs
- THEN it sends `PATCH /v1.0/me/mailFolders/F1` with that body and prints `{"response": <the folder Graph answered>}`

### Requirement: Protocol listings print the provider's payload
The protocol commands whose provider answers JSON SHALL print the provider's payload as received (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful*): a listing each item as the provider sent it, every property kept, under a collection key, and the other keys of the answer beside it as the provider spells them, the provider's cursor alone lifted into `nextPage` ("Listings resume from their cursor"); a get of one resource that resource as the provider sent it, not wrapped in a list. The text form SHALL read what it shows from those items.

- Gmail: `gmail messages list`, `threads list`, `drafts list` and `history list` SHALL print Gmail's page, `messages`, `threads`, `drafts` or `history` (`[]` when Gmail leaves the key out of an empty page) beside `resultSizeEstimate` or `historyId`, `nextPageToken` lifted into `nextPage`. A history record SHALL print as Gmail sent it (`messages`, `messagesAdded`, `messagesDeleted`, `labelsAdded`, `labelsRemoved`, each holding Gmail's message objects). `gmail labels list` and the settings lists `gmail settings sendas list`, `delegates list`, `forwarding-addresses list` and `filters list`, which Gmail answers whole, SHALL print Gmail's answer, `labels`, `sendAs`, `delegates`, `forwardingAddresses` or `filter` (`[]` when Gmail leaves the key out), with no `nextPage`. `gmail labels get <ID>` SHALL take one id and print the label as Gmail sent it.
- Microsoft Graph: `msgraph messages list` SHALL print `messages` beside `@odata.count` when Graph sent one; `msgraph mail-folders list|child-folders` `folders`; `msgraph attachments list` `attachments`, `contentBytes` included, across every page it follows; the items read through io-msgraph's `page_get`, the first page's link built as io-msgraph builds it, so a `$select` property io-msgraph does not model still prints. `msgraph mail-folders get <ID>` SHALL take one id and print the folder as Graph sent it, and `msgraph profile get` the signed-in user (`GET /me`, or `users/<id>` under `msgraph.user-id`) as Graph sent it.
- JMAP: `jmap email query|get`, `jmap mailbox query|get` and `jmap submission query|get` SHALL send the `Foo/query` chained into its `Foo/get` in one request, or the `Foo/get` alone, through io-jmap's raw send, and print the get's `list` under `emails`, `mailboxes` or `submissions`, beside the query's `queryState`, `canCalculateChanges`, `position` and `total` and the get's `state`, `ids` and `accountId` left out, `notFound` read into `errors` ("A command acting on several items lists what failed"), and `nextPage` the next page number. `jmap thread get` and `jmap identity get` SHALL send their `Foo/get` the same way, an identity under the submission capability and every identity when given no id (`ids` `null`), and print `threads` or `identities` beside the get's `state`. `jmap submission create` SHALL send its `EmailSubmission/set` the same way and print the submission as the server answered its creation (the properties the server set, `id` among them) under `submissions`, beside the set's `oldState` and `newState`, a `notCreated` printing as its refusal. A method answering `error` SHALL print that error as the command's refusal, coded as the typed calls code it (*A protocol refusal is a result*).

#### Scenario: A Gmail page keeps its estimate
- GIVEN a Gmail account answering `users.messages.list` with `{"messages":[{"id":"m1","threadId":"t1"}],"nextPageToken":"p2","resultSizeEstimate":3}`
- WHEN `himalaya --json gmail messages list -s 1` runs
- THEN it prints `{"messages":[{"id":"m1","threadId":"t1"}],"resultSizeEstimate":3,"nextPage":"p2"}`

#### Scenario: A Graph property no type models
- GIVEN a Graph account whose messages carry `importance`
- WHEN `himalaya --json msgraph messages list --select id,importance` runs
- THEN each message prints its `importance`, and `--page-token <nextPage>` given beside `--top 1` is refused, exit 2

#### Scenario: One label by id
- GIVEN a Gmail account holding the label `L1`
- WHEN `himalaya --json gmail labels get L1` runs
- THEN it prints the label object Gmail answered, every property kept, where it printed `{"labels":[...]}`

### Requirement: A command acting on several items lists what failed
`envelope list`, `envelope search`, `pimdir search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1, and so SHALL a failure of the exchange itself met part-way (the connection, a batch answer that does not parse), only the server's refusal of an item carrying on. A listing reading the whole mailbox before cutting its page (Maildir, mbox, a JMAP search rechecked client-side) SHALL print its errors on its first page only. The commands acting on one item (`message read`, `message add`, `message send`, the composers, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`, but the JMAP gets and the queries chaining one: `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` SHALL print one entry per id the server did not find, coded `message-not-found`, `mailbox-not-found`, `thread-not-found`, `identity-not-found` or `submission-not-found`, after the ones they list, the objects listed printing as the server sent them ("Protocol listings print the provider's payload"). `jmap submission create`, printing the shape of a get, SHALL print `errors` as `[]`. The JMAP sets acting on several objects, `jmap email delete|update|copy`, `jmap mailbox destroy`, `jmap identity delete` and `jmap submission cancel`, SHALL print `{ids, errors}`: the ids of the objects the server applied the write to (for a copy, the ids of the copies), then one item error per object it refused, keyed by the id given and coded by its SetError type, a `notFound` `message-not-found`, `mailbox-not-found`, `identity-not-found` or `submission-not-found`.

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
