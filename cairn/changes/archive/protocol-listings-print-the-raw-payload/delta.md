---
cairn: delta
change: protocol-listings-print-the-raw-payload
---

# Delta

## ADDED Requirements

### Requirement: Protocol listings print the provider's payload
The protocol listings whose provider answers JSON SHALL print the provider's payload as received (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful*): each item as the provider sent it, every property kept, under a collection key, and the other keys of the answer beside it as the provider spells them, the provider's cursor alone lifted into `nextPage` ("Listings resume from their cursor"). The text form SHALL read what it shows from those items.

- Gmail: `gmail messages list`, `threads list`, `drafts list` and `history list` SHALL print Gmail's page, `messages`, `threads`, `drafts` or `history` (`[]` when Gmail leaves the key out of an empty page) beside `resultSizeEstimate` or `historyId`, `nextPageToken` lifted into `nextPage`. A history record SHALL print as Gmail sent it (`messages`, `messagesAdded`, `messagesDeleted`, `labelsAdded`, `labelsRemoved`, each holding Gmail's message objects).
- Microsoft Graph: `msgraph messages list` SHALL print `messages` beside `@odata.count` when Graph sent one; `msgraph mail-folders list|child-folders` `folders`; `msgraph attachments list` `attachments`, `contentBytes` included, across every page it follows; the items read through io-msgraph's `page_get`, the first page's link built as io-msgraph builds it, so a `$select` property io-msgraph does not model still prints. `msgraph mail-folders get`, printing the shape of a listing, SHALL print each folder as io-msgraph reads it.
- JMAP: `jmap email query|get`, `jmap mailbox query|get` and `jmap submission query|get` SHALL send the `Foo/query` chained into its `Foo/get` in one request, or the `Foo/get` alone, through io-jmap's raw send, and print the get's `list` under `emails`, `mailboxes` or `submissions`, beside the query's `queryState`, `canCalculateChanges`, `position` and `total` and the get's `state`, `ids` and `accountId` left out, `notFound` read into `errors` ("A command acting on several items lists what failed"), and `nextPage` the next page number. A method answering `error` SHALL fail the command with that error, coded as the typed calls code it. `jmap submission create`, printing the shape of a get, SHALL print each created submission as io-jmap reads it.

#### Scenario: A Gmail page keeps its estimate
- GIVEN a Gmail account answering `users.messages.list` with `{"messages":[{"id":"m1","threadId":"t1"}],"nextPageToken":"p2","resultSizeEstimate":3}`
- WHEN `himalaya --json gmail messages list -s 1` runs
- THEN it prints `{"messages":[{"id":"m1","threadId":"t1"}],"resultSizeEstimate":3,"nextPage":"p2"}`

#### Scenario: A Graph property no type models
- GIVEN a Graph account whose messages carry `importance`
- WHEN `himalaya --json msgraph messages list --select id,importance` runs
- THEN each message prints its `importance`, and `--page-token <nextPage>` given beside `--top 1` is refused, exit 2

## MODIFIED Requirements

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page follows the one printed, `null` once the page reaches the end of the listing, an exactly full last page included, and SHALL take it back through `--page-token`, which wins over `--page`. Whether a page follows SHALL come from the listing's own exchanges: one does when the page ends before the count the backend holds (the messages Maildir and mbox list or match, the `EXISTS` of IMAP's `SELECT` for a listing and the UIDs its `SORT` answers for a search, the `total` of JMAP's `Email/query`, the items pimdir counts for a listing), when pimdir's search finds one hit past the page, when the listing of Gmail or Graph holds one message past the page, which the page asks along (`maxResults` or `$top` one longer, a later Gmail page and a Graph search reading the pages before it in as few calls as the largest page allows, the next page token or link followed only while the listing runs short of that message), both answering a next page token or link after an exactly full last page too, and, on a JMAP server computing no `total`, when the page comes back full. Pages SHALL be numbered from 1: `--page 0` and a page token `0` are refused, `--page-size 0` listing the whole mailbox. `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders` SHALL print Graph's next link (`@odata.nextLink`) as `nextPage`, as Graph gives it, a `$search` page's included, and not beside it, `null` without one or when it leads back to the page asked (`--top 0`); `--page-token` SHALL follow it as is, the query it carries standing for the other options, so a query option given beside it (`--folder`, `--top`, `--skip`, `--filter`, `--search`, `--orderby`, `--select`, `--count`, `--hidden`, the parent folder of `child-folders`) SHALL be refused as a usage error, exit 2, before connecting; a link off the origin of Graph's API (`https://graph.microsoft.com`) being refused before any request, the bearer token never leaving it. `msgraph attachments list` SHALL follow every next link, so it never stops early and prints no cursor. `msgraph messages list --search` SHALL quote its value for Graph, its double quotes escaped, unless it opens on a double quote, and `--count` SHALL keep Graph's `@odata.count` as Graph sends it (`Count:` in text), absent without `--count` or beside `--search`, Graph ignoring `$count` there. The Gmail listings (`gmail messages list`, `threads list`, `drafts list`, `history list`) SHALL print Gmail's `nextPageToken` as `nextPage`, and not beside it, and take it back through `--page-token`; Gmail's other lists hold no cursor. `jmap email query`, `jmap mailbox query` and `jmap submission query` SHALL print `nextPage` beside their collection the same way, a page following while the query's `total` lies past it, or, on a server computing none, while it comes back full, and SHALL take it back through `--page-token`, a token that is no page number from 1 being refused; `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create`, printing the same shape, SHALL print `nextPage` as `null`.

#### Scenario: A second page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 1` runs, then `himalaya --json envelope list -s 1 --page-token 2`
- THEN the first prints one envelope and `"nextPage":"2"`, the second the other envelope and `"nextPage":null`

#### Scenario: An exactly full last page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 2` runs
- THEN it prints both envelopes, `"nextPage":null` and `"total":2`, and its text prints `Page:` `1, envelopes 1 to 2, the last` with no `Next page:` line

#### Scenario: A JMAP query reaching its total
- GIVEN a JMAP account holding two mailboxes, its server computing `total`
- WHEN `himalaya --json jmap mailbox query -s 2` runs
- THEN it prints both mailboxes and `"nextPage":null`, and its text prints `Page:` `1, mailboxes 1 to 2, the last` with no `Next page:` line

#### Scenario: A Graph search resumed from its next link
- GIVEN a Microsoft Graph account whose search for `report` matches three messages
- WHEN `himalaya --json msgraph messages list --search report --top 1` runs, then `himalaya --json msgraph messages list --page-token <nextPage>` with the `nextPage` it printed
- THEN the first prints Graph's next link, `$skiptoken` and all, as `nextPage` and no `@odata.nextLink`, and the second sends that link as is and prints the second message and the next link after it

### Requirement: A command acting on several items lists what failed
`envelope list`, `envelope search`, `pimdir search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1, and so SHALL a failure of the exchange itself met part-way (the connection, a batch answer that does not parse), only the server's refusal of an item carrying on. A listing reading the whole mailbox before cutting its page (Maildir, mbox, a JMAP search rechecked client-side) SHALL print its errors on its first page only. The commands acting on one item (`message read`, `message add`, `message send`, the composers, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`, but the JMAP gets and the queries chaining one: `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` SHALL print one entry per id the server did not find, coded `message-not-found`, `mailbox-not-found`, `thread-not-found`, `identity-not-found` or `submission-not-found`, after the ones they list, the objects listed printing as the server sent them ("Protocol listings print the provider's payload"). `jmap submission create`, printing the shape of a get, SHALL print `errors` as `[]`.

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

## REMOVED Requirements

### Requirement: Gmail history retains added-message details

Replaced by *Protocol listings print the provider's payload*: a history record prints as Gmail sent it.
