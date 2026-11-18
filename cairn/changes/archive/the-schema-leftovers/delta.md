---
cairn: delta
change: the-schema-leftovers
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: ManageSieve commands

When built with the `sieve` feature, the protocol-specific API SHALL expose `sieve capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Every command SHALL print a dedicated serializable output type: `activate`, `rename` and `delete` the receipt `{id}` naming the script, `activate` refusing an empty name as a usage error, exit 2, before connecting (RFC 5804 reads `SETACTIVE ""` as a deactivation), `deactivate` `{active: null}`, `check` and `put` `{name, warnings}`, `warnings` being the lines of the text a server attaches to an accepted script, one entry per line, `[]` when it sends none, and `name` `null` for a check. `put` and `check` SHALL accept script bytes from a file, inline arguments, or stdin.

The protocol SHALL live in io-managesieve, so this repository holds a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, exactly as the other protocol modules do.

### Requirement: pimdir lists each mailbox with its coverage
`pimdir mailbox list` SHALL list the account's mailboxes sorted by id, each as `{"id", "name", "role", "total", "unread", "coverage", "round"}`: `role` as `mailbox list` shows it (`null` when none), `total` and `unread` counted in the store, `coverage` the collection's coverage (pimdir STORAGE §14.1, the narrowest of its sources') as `{"since", "until", "closedAt"}`, `closedAt` when the round leaving it closed, and `round` a round a source has under way as `{"since", "until", "startedAt"}`. Bounds are RFC 3339 instants, `null` for an open end. `coverage` SHALL be `null` until every source of the collection closed a round on it, and on a store its owner has not reconciled; `round` SHALL be `null` when no round is under way. The plain list SHALL show the coverage and the round as their days (`since 2026-09-07`, `all`). The unified `mailbox list` SHALL carry none of it.

#### Scenario: A mailbox synced from a date
- GIVEN a pimdir account whose `imap` source closed a round on `imap/INBOX` over the mail since 2026-09-07
- WHEN `himalaya --json pimdir mailbox list` runs
- THEN `imap/INBOX` carries `"coverage":{"since":"2026-09-07T00:00:00Z","until":null,"closedAt":…}`, and a mailbox no round closed on carries `"coverage":null`

### Requirement: pimdir searches through the store's index
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-draft-13 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. The answer SHALL be the store as of the owner's last refresh: a queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort date|name|relevance` orders the hits (date by default), `--page` and `--page-size` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date as `firstDate` and `lastDate`, and its hits. Under `--json` the page SHALL print `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`: `nextPage` the number of the next page, as a string, while hits or threads follow the page, `null` once the answer is complete, which `--page-token` SHALL take back, winning over `--page`, a token that is no page number from 1 being refused, coded `page-token-invalid`. A hit whose message the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other hits listed (a thread without that hit) and the exit status 3; in text the entries print after the items, as every list prints them.

A `date:` term's years, months and days, `today` and `yesterday` SHALL be read in the local zone (SEARCH §8.1), in `pimdir search`, `count` and `tag` alike: `TZ`, then the system's zone, by its IANA name. A zone with no IANA name, a POSIX `TZ` string or a system zone that cannot be named, or no zone at all, SHALL leave them in UTC with one warning.

#### Scenario: A body match
- GIVEN a pimdir store whose index holds a message whose body says `quarterly`
- WHEN `himalaya pimdir search quarterly` runs
- THEN the message is listed with `matched` `body` and a snippet holding `quarterly`

#### Scenario: A staged message is not listed
- GIVEN a message added to a pimdir mailbox and still in the queue
- WHEN `himalaya pimdir search` runs a query its body matches
- THEN it is not listed until the owner applies the add and refreshes the index

#### Scenario: A second page of a search
- GIVEN a pimdir index holding three messages `report` matches
- WHEN `himalaya --json pimdir search -s 2 report` runs, then `himalaya --json pimdir search -s 2 --page-token 2 report`
- THEN the first prints two hits and `"nextPage":"2"`, the second the third hit and `"nextPage":null`

#### Scenario: A hit the store cannot read
- GIVEN a pimdir index holding three messages `report` matches, the store failing to read the row of one
- WHEN `himalaya --json pimdir search report` runs
- THEN it prints the two other hits and `"errors":[{"id":"<its id>","code":"message-unreadable",…}]`, and the exit status is 3

#### Scenario: A day is the local day
- GIVEN `TZ=Europe/Paris` and an indexed message received at 00:30 on 2026-11-07 in that zone
- WHEN `himalaya pimdir search date:2026-11-07` runs
- THEN the message is listed, and `date:2026-11-06` does not list it

### Requirement: A command acting on several items lists what failed
`envelope list`, `envelope search`, `pimdir search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1, and so SHALL a failure of the exchange itself met part-way (the connection, a batch answer that does not parse), only the server's refusal of an item carrying on. A listing reading the whole mailbox before cutting its page (Maildir, mbox, a JMAP search rechecked client-side) SHALL print its errors on its first page only. The commands acting on one item (`message read`, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`, but `message add`, `message send` and the composers, whose `errors` lists what failed after the message went out ("Saving a sent message follows the send"), but the JMAP gets and the queries chaining one: `jmap email get|query`, `jmap mailbox get|query`, `jmap thread get`, `jmap identity get` and `jmap submission get|query` SHALL print one entry per id the server did not find, coded `message-not-found`, `mailbox-not-found`, `thread-not-found`, `identity-not-found` or `submission-not-found`, after the ones they list, the objects listed printing as the server sent them ("Protocol listings print the provider's payload"). `jmap submission create`, printing the shape of a get, SHALL print `errors` as `[]`. `jmap email parse` SHALL print `{bodies, errors}`, each text body `{blobId, body}` beside the blob it came from, and one entry per blob the server did not find, coded `message-not-found`, or could not parse, coded `message-unreadable`. The JMAP sets acting on several objects, `jmap email delete|update|copy`, `jmap mailbox destroy`, `jmap identity delete` and `jmap submission cancel`, SHALL print `{ids, errors}`: the ids of the objects the server applied the write to (for a copy, the ids of the copies), then one item error per object it refused, keyed by the id given and coded by its SetError type, a `notFound` `message-not-found`, `mailbox-not-found`, `identity-not-found` or `submission-not-found`.

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

