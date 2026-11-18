---
cairn: delta
change: single-reads-print-raw
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Data commands serialize their data
Every command SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key; a sentence is the text form of that data, never a JSON field. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. A protocol read of one resource SHALL print it as the provider sent it ("Protocol listings print the provider's payload"), its text form listing what it shows from it: `gmail messages get` the message Gmail answered at the format asked, `payload` included under metadata and full, `gmail threads get` the thread with each message as Gmail answered it, its text form listing each leaf part of a full payload with its MIME type, file name, size and attachment id. A Gmail settings write SHALL print Gmail's answer to its `PUT` as Gmail sent it, the shape of the matching `get`. A Gmail list SHALL print the provider's collection under its own key (`delegates`, `filter`, `forwardingAddresses`), and `gmail messages list` its `messages`.

### Requirement: Gmail header selection applies to every format
`gmail messages get --header` and `gmail threads get --header` SHALL narrow the headers the text form renders whatever the requested format, and SHALL send Gmail's `metadataHeaders` under the metadata format, which Gmail honours there alone. The JSON SHALL stay the payload Gmail answered, its headers under `payload.headers`. Matching is case-insensitive, order and repeats are preserved, and passing no `--header` renders every header. Headers are read from the top-level payload part, where Gmail puts the RFC 5322 headers.

### Requirement: Protocol listings print the provider's payload
The protocol commands whose provider answers JSON SHALL print the provider's payload as received (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful*): a listing each item as the provider sent it, every property kept, under a collection key, and the other keys of the answer beside it as the provider spells them, the provider's cursor alone lifted into `nextPage` ("Listings resume from their cursor"); a get of one resource that resource as the provider sent it, not wrapped in a list. The text form SHALL read what it shows from those items.

- Gmail: `gmail messages list`, `threads list`, `drafts list` and `history list` SHALL print Gmail's page, `messages`, `threads`, `drafts` or `history` (`[]` when Gmail leaves the key out of an empty page) beside `resultSizeEstimate` or `historyId`, `nextPageToken` lifted into `nextPage`. A history record SHALL print as Gmail sent it (`messages`, `messagesAdded`, `messagesDeleted`, `labelsAdded`, `labelsRemoved`, each holding Gmail's message objects). `gmail labels list` and the settings lists `gmail settings sendas list`, `delegates list`, `forwarding-addresses list` and `filters list`, which Gmail answers whole, SHALL print Gmail's answer, `labels`, `sendAs`, `delegates`, `forwardingAddresses` or `filter` (`[]` when Gmail leaves the key out), with no `nextPage`. `gmail labels get <ID>` SHALL take one id and print the label as Gmail sent it, and so SHALL `gmail profile get` (`users.getProfile`), `gmail drafts get`, `gmail messages get`, `gmail threads get` and the settings reads (`gmail settings vacation|imap|pop|auto-forwarding|language get`, `filters|forwarding-addresses|delegates|send-as get`) print the resource Gmail answered.
- Microsoft Graph: `msgraph messages list` SHALL print `messages` beside `@odata.count` when Graph sent one; `msgraph mail-folders list|child-folders` `folders`; `msgraph attachments list` `attachments`, `contentBytes` included, across every page it follows; the items read through io-msgraph's `page_get`, the first page's link built as io-msgraph builds it, so a `$select` property io-msgraph does not model still prints. `msgraph mail-folders get <ID>` SHALL take one id and print the folder as Graph sent it, `msgraph profile get` the signed-in user (`GET /me`, or `users/<id>` under `msgraph.user-id`) and `msgraph messages get` the message, each as Graph sent it.
- JMAP: `jmap email query|get`, `jmap mailbox query|get` and `jmap submission query|get` SHALL send the `Foo/query` chained into its `Foo/get` in one request, or the `Foo/get` alone, through io-jmap's raw send, and print the get's `list` under `emails`, `mailboxes` or `submissions`, beside the query's `queryState`, `canCalculateChanges`, `position` and `total` and the get's `state`, `ids` and `accountId` left out, `notFound` read into `errors` ("A command acting on several items lists what failed"), and `nextPage` the next page number. `jmap thread get` and `jmap identity get` SHALL send their `Foo/get` the same way, an identity under the submission capability and every identity when given no id (`ids` `null`), and print `threads` or `identities` beside the get's `state`. `jmap vacation-response get` SHALL send its `VacationResponse/get` the same way, under the vacation response capability, and print the singleton as the server sent it, `{"id":"singleton","isEnabled":false}` when the server holds none. `jmap submission create` SHALL send its `EmailSubmission/set` the same way and print the submission as the server answered its creation (the properties the server set, `id` among them) under `submissions`, beside the set's `oldState` and `newState`, a `notCreated` printing as its refusal. A method answering `error` SHALL print that error as the command's refusal, coded as the typed calls code it (*A protocol refusal is a result*).

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

