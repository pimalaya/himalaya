---
cairn: delta
change: protocol-commands-print-the-raw-payload
---

# Delta

## ADDED Requirements

### Requirement: The Gmail and Graph escape hatches are requests
When built with the `gmail` or `msgraph` feature, `gmail request <METHOD> <PATH> [JSON]` and `msgraph request <METHOD> <PATH> [JSON]` SHALL send one HTTP request under the account's bearer token, as Cardamum's `gpeople request` and `msgraph request` do: METHOD `get`, `post`, `patch` or `delete`, `put` too on Gmail, in any case, another refused before connecting; PATH joined onto `https://gmail.googleapis.com/gmail/v1/` or `https://graph.microsoft.com/v1.0/`, a leading `/` dropped, or a full URL taken as it is; JSON the body of a `post`, `put` or `patch`, `null` when absent. They SHALL print `{response}`, the body the provider answered as it sent it, `null` for an empty one, its text form the body pretty-printed, nothing for an empty one. A refusal SHALL fail as the typed commands' refusals do.

#### Scenario: A Graph patch
- GIVEN a Microsoft Graph account
- WHEN `himalaya --json msgraph request patch /me/mailFolders/F1 '{"displayName":"Work"}'` runs
- THEN it sends `PATCH /v1.0/me/mailFolders/F1` with that body and prints `{"response": <the folder Graph answered>}`


## MODIFIED Requirements

### Requirement: The JMAP escape hatch is a request
When built with the `jmap` feature, `jmap request <JSON>` SHALL send one JMAP request (RFC 8620 section 3.3) given as a whole request object, `{"using": [...], "methodCalls": [...]}`: a path to a JSON file, the JSON itself, or `-` for stdin (refused inside a session as `stdin-taken`), `using` the core and mail capabilities when absent, the calls sent as written. It SHALL print `{response}`, the server's `{methodResponses, createdIds, sessionState}` as it answered them, its text form that object pretty-printed, as Cardamum's `jmap request` does. `request` is the escape hatch of an HTTP API across the family, `raw` the one of a line protocol (pimalaya-cli cairn/spec/commands.md). `jmap query [--using <URN>]... [METHOD_CALLS]...`, its former form, SHALL stay, hidden from the help and the manual, under its schema key `himalaya-jmap-query`: it sends one request whose `methodCalls` is the JSON array given, read from stdin when it is `-` or absent (refused inside a session as `stdin-taken`), whose `using` is the core and mail capabilities plus each `--using`, and whose calls naming no `accountId` get the session's primary mail account, and prints `{methodResponses}`, each response `[name, arguments, callId]` as the server answered it, its text form the same array.

#### Scenario: The former form
- GIVEN a JMAP account
- WHEN `himalaya --json jmap query '[["Mailbox/get", {}, "c0"]]'` runs, then `himalaya --json jmap request '{"methodCalls": [["Mailbox/get", {"accountId": "a1"}, "c0"]]}'`
- THEN the first prints `{"methodResponses": [...]}` and the second `{"response": {"methodResponses": [...], "createdIds": null, "sessionState": "..."}}`, and `himalaya jmap --help` lists `request` alone


### Requirement: Protocol listings print the provider's payload
The protocol commands whose provider answers JSON SHALL print the provider's payload as received (pimalaya-cli cairn/spec/output.md, *Protocol commands are raw-faithful*): a listing each item as the provider sent it, every property kept, under a collection key, and the other keys of the answer beside it as the provider spells them, the provider's cursor alone lifted into `nextPage` ("Listings resume from their cursor"); a get of one resource that resource as the provider sent it, not wrapped in a list. The text form SHALL read what it shows from those items.

- Gmail: `gmail messages list`, `threads list`, `drafts list` and `history list` SHALL print Gmail's page, `messages`, `threads`, `drafts` or `history` (`[]` when Gmail leaves the key out of an empty page) beside `resultSizeEstimate` or `historyId`, `nextPageToken` lifted into `nextPage`. A history record SHALL print as Gmail sent it (`messages`, `messagesAdded`, `messagesDeleted`, `labelsAdded`, `labelsRemoved`, each holding Gmail's message objects). `gmail labels list` and the settings lists `gmail settings sendas list`, `delegates list`, `forwarding-addresses list` and `filters list`, which Gmail answers whole, SHALL print Gmail's answer, `labels`, `sendAs`, `delegates`, `forwardingAddresses` or `filter` (`[]` when Gmail leaves the key out), with no `nextPage`. `gmail labels get <ID>` SHALL take one id and print the label as Gmail sent it.
- Microsoft Graph: `msgraph messages list` SHALL print `messages` beside `@odata.count` when Graph sent one; `msgraph mail-folders list|child-folders` `folders`; `msgraph attachments list` `attachments`, `contentBytes` included, across every page it follows; the items read through io-msgraph's `page_get`, the first page's link built as io-msgraph builds it, so a `$select` property io-msgraph does not model still prints. `msgraph mail-folders get <ID>` SHALL take one id and print the folder as Graph sent it, and `msgraph profile get` the signed-in user (`GET /me`, or `users/<id>` under `msgraph.user-id`) as Graph sent it.
- JMAP: `jmap email query|get`, `jmap mailbox query|get` and `jmap submission query|get` SHALL send the `Foo/query` chained into its `Foo/get` in one request, or the `Foo/get` alone, through io-jmap's raw send, and print the get's `list` under `emails`, `mailboxes` or `submissions`, beside the query's `queryState`, `canCalculateChanges`, `position` and `total` and the get's `state`, `ids` and `accountId` left out, `notFound` read into `errors` ("A command acting on several items lists what failed"), and `nextPage` the next page number. `jmap thread get` and `jmap identity get` SHALL send their `Foo/get` the same way, an identity under the submission capability and every identity when given no id (`ids` `null`), and print `threads` or `identities` beside the get's `state`. `jmap submission create` SHALL send its `EmailSubmission/set` the same way and print the submission as the server answered its creation (the properties the server set, `id` among them) under `submissions`, beside the set's `oldState` and `newState`, a `notCreated` failing the command as before. A method answering `error` SHALL fail the command with that error, coded as the typed calls code it.

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


### Requirement: An option after the words is an option
A positional taking several words (the query of `pimdir search`, `pimdir count` and `pimdir tag`, the message `MessageArg` resolves, the script of `sieve put` and `sieve check`, the method calls of `jmap query`) SHALL leave every option an option wherever it stands, before, between or after the words, the global `--json` included, as `envelope search` does. A word opening on a hyphen SHALL go after `--`: a pimdir negation is `-- -from:alice`, or `not from:alice`; given before `--`, it is refused as an unknown option.

#### Scenario: A dry run after the query
- GIVEN a pimdir store whose index holds messages from `alice@x.org`
- WHEN `himalaya pimdir tag --add flagged from:alice@x.org --dry-run` runs
- THEN it reports the plan for the query `from:alice@x.org` and stages nothing

#### Scenario: An option after a pimdir query
- GIVEN a pimdir index holding three messages `report` matches
- WHEN `himalaya pimdir search report -s 2 --json` runs
- THEN it prints the first two hits as JSON with `"nextPage":"2"`


In the requirement on text forms, `gmail labels get` and `msgraph mail-folders get` print fields (`Id:`, `Type:`, `Messages:`, `Name:`; `Id:`, `Name:`, `Parent:`, `Messages:`, `Child folders:`) where they printed a listing.

## REMOVED Requirements

None.
