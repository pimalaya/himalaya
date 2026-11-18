---
cairn: delta
change: the-pimdir-refusals-are-coded
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: pimdir reads one account
The pimdir backend SHALL show the collections of one account (pimdir STORAGE §9.2), `pimdir.account` naming it. Unset, it is derived from the accounts holding mail: a store where one account, or one ungrouped set, holds mail is read as that one, whatever contacts or calendars other accounts hold, and a store where several hold mail is refused coded `store-account-ambiguous`, naming them, rather than guessing one and showing the wrong mailbox set. A `pimdir.account` the store holds no collection of SHALL be refused coded `store-account-unknown`, rather than read as an empty account, `account check` included. A root holding no store SHALL be refused coded `store-uncreated`, as the other pimdir programs refuse it. A mailbox the account does not hold SHALL be refused coded `mailbox-not-found`, and a message id that is no number coded `message-not-found`.

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL list the account's queued creations (`add`) and sends (`submit`), and its parked rows of any kind, mailbox by mailbox in store order, or those of the one mailbox `-m/--mailbox` names (a name, alias, role or id), where the kind-agnostic `pimdir` binary can only print ids and hashes.

Under `--json` it SHALL print `{"messages": [<row>], "parked": [<row>], "refused": [<refusal>]}`, the pending rows then the parked ones, each in append order and in the family's one queue row (pimalaya-cli cairn/spec/output.md, *A queue row keeps one shape*), every key printed: `{"row", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject"}`. `row` is the queue row id `queue show` and `queue cancel` take; `kind` the row's action as the store holds it, read from io-pimdir (`PimdirAction::kind` for a pending row, `PimdirParkedAction::action` for a parked one), never re-spelled; `collection` the mailbox's id as `mailbox list` prints it; `linkId` the link id of the message an `add` creates, the bare `Message-ID` its receipt printed, `null` for any other row, a `submit` among them, the store creating no message for it; `itemId` the public id of the stored message a `set-flags`, `move`, `copy`, `remove` or `update` acts on, as `envelope list` prints it, `null` for any other row; `producer` the process that staged the row; `queuedAt` the instant the store stamped it; `attempts` how many times the sync engine tried it, an integer from 0 (`uint64`, minimum 0, in its JSON Schema); `unconfirmed` `true` for a row whose outcome is unknown (pimdir STORAGE §15.2); `error` `null` while the row is pending and the store's text once it is parked; `subject` the subject of the body the row pins, else of the message it acts on, a parked `submit` naming no body giving its payload's subject, `null` when there is none to read. The queued action carries no summary, so a subject SHALL be derived from the body the action pins, through the same derivation the owner applies. A parked payload that does not decode addresses nothing: `linkId`, `itemId` and `subject` are `null`. No placeholder SHALL stand in for the id a queued message lacks (no `""`, `0` nor row id).

`refused` SHALL list every refusal standing in what is listed, the account or the one mailbox, (io-pimdir's `PimdirReader::refusals`, pimdir STORAGE §15.4), per message and source, `{"collection", "itemId", "source", "reason", "refusedAt"}`: a change the store applied that a source's server refused for good, which the sync engine holds until the message is changed again or removed. The text form SHALL list each after the parked rows, `refused` in the state column, its instant, the message id, then the source and the reason; `Cancel:` SHALL print only when a pending or parked row is listed, a refusal being no row to cancel. The header SHALL name the mailbox only when `--mailbox` narrows the listing.

#### Scenario: A queued send
- GIVEN a pimdir account holding `imap/INBOX` and `imap/Sent`, and a send queued on `imap/Sent`
- WHEN `himalaya --json pimdir queue list` runs, then `himalaya --json pimdir queue list -m imap/INBOX`
- THEN the first lists the `submit` row of `imap/Sent`, and the second lists no row

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output: `{"row", "kind", "unconfirmed"}`. An unconfirmed row (pimdir STORAGE §15.2) is cancelled as a parked one, which undoes nothing its performer sent: the prompt and the plain output SHALL say it was sent and may have taken effect, and `unconfirmed` is `true`. A row anchored on a collection that is not one of the account's mailboxes (another account's, or an address book or calendar another app of the store queued on), or one neither pending nor parked, SHALL be refused before anything is opened or asked, and stays as it was, coded `row-not-found` unless it was applied. With no terminal to answer and no `--yes`, the cancel SHALL be refused coded `confirmation-required` before anything is opened.

Taking the owner role briefly is what cancelling costs (pimdir STORAGE §15.5); the backend read and write paths SHALL NOT reach it. The cancel SHALL open the store only as it stands (io-pimdir `open_current`): it SHALL NOT create, migrate nor reconcile the schema. A store of another draft SHALL be refused as `store-version` and a damaged one as `store-corrupt`, each left as it is. A store another process owns SHALL be refused immediately, saying a sync is running and that the action may already have been applied.

#### Scenario: A queued creation is listed from its body
- GIVEN a queued `add` of the message `d1@x.org` pinning a body whose `Subject:` is `hi`
- WHEN the queue is listed with `--json`
- THEN its row prints `"kind":"add"`, `"linkId":"d1@x.org"`, `"itemId":null`, `"error":null` and `"subject":"hi"`, and no `send` nor `envelope`

#### Scenario: A parked move names its message
- GIVEN a `move` of message 7, whose subject is `hi`, the sync engine parked
- WHEN the queue of its mailbox is listed with `--json`
- THEN the row is under `parked` with `"kind":"move"`, `"itemId":"7"`, `"linkId":null`, `"subject":"hi"` and the store's error text

#### Scenario: A cancel on a store of another draft
- GIVEN a store whose `store_meta.format_version` names another draft
- WHEN `himalaya pimdir queue cancel` runs on one of its rows
- THEN the command fails with the code `store-version`, pointing at neverest's MIGRATION.md, and neither the store's stamp nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued

#### Scenario: A flag change a server refused
- GIVEN a pimdir flag change the sync engine applied, then the IMAP source's server refused for good
- WHEN `himalaya --json pimdir queue show <ROW>` and `pimdir queue list` run on its row and its mailbox
- THEN the row reads `applied` on its message with one entry of `refusals` naming `imap` and the server's reason, and the mailbox lists the message under `refused`

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary and pimalaya-cli's `usage-invalid`, the item error of an `errors` list under `himalaya-item-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.

| Code | Raised when |
|---|---|
| `usage-invalid` | a command line, a REPL line or an MCP tool's arguments the grammar refuses (an unknown flag, command or argument, a missing argument, a bad value); raised by pimalaya-cli beside Himalaya's codes; nothing runs |
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |
| `message-too-complex` | a message is nested deeper than 8 levels, holds more than 200 parts or 500 header lines, or passes the MIME parser's own caps; it is refused whole |
| `stdin-taken` | a command inside a `repl` session would read its input from stdin, which carries the session's commands; pass it as an argument or a file path |
| `attachment-too-large` | an attachment is larger than `himalaya mcp --max-attachment-size`; one given inline is refused before it is decoded, one a tool reads before it is encoded; nothing is done, nothing of it returned |
| `attachment-invalid` | an `attach-data` item given to an MCP composer is malformed (no file name, both or neither of `data` and `text`, bad base64, a bad `mimeType`); nothing is composed |
| `search-invalid` | an `envelope search` query does not parse; nothing is searched |
| `search-unsupported` | an `envelope search` clause the account's backend cannot search, quoted as written; nothing is searched, no clause dropped |
| `capability-missing` | the account's backend does not serve the verb, or the option of it asked for, as its capability table declares or its server advertises; nothing is done, the message naming the capability and what serves it instead |
| `mailbox-not-found` | the mailbox named does not exist (Maildir, mbox, IMAP, pimdir); nothing is done; an `errors` entry of `jmap mailbox get` and `query` for an id the JMAP server did not find |
| `mailbox-duplicate` | a create names a mailbox that exists: a Maildir folder or an mbox file already there, a mailbox a pimdir store lists or has its creation queued, an IMAP `CREATE` refused `ALREADYEXISTS` or refused with no code while `LIST` shows the mailbox, a JMAP `Mailbox/set` refused while a sibling holds the name, a Gmail 409, a Graph 409 `ErrorFolderExists` or a child of that name under the parent; nothing is created nor staged |
| `message-not-found` | a message id given is not in the mailbox: an `errors` entry beside what was done; for the raw `maildir` and `mbox` commands, one of them is not, or the one message a command reads is not (every backend), and nothing is done; an `errors` entry of `jmap email get` and `query` for an id the JMAP server did not find |
| `message-unreadable` | the backend answered for a message the client could not read: its FETCH line or one of its items did not decode (IMAP), its object did not deserialize (JMAP), its file could not be read (Maildir), it holds no header field (Maildir, mbox), the store could not read its row (`pimdir search`); an `errors` entry, the message listed with what was read when there is any, or the failure of a `message read` |
| `mailbox-unreadable` | a mailbox was listed but could not be read: its LIST or STATUS line did not decode (IMAP), its folder could not be read (Maildir), its counts could not be fetched; an `errors` entry |
| `thread-not-found` | an `errors` entry of `jmap thread get` for an id the JMAP server did not find |
| `identity-not-found` | an `errors` entry of `jmap identity get` for an id the JMAP server did not find |
| `submission-not-found` | an `errors` entry of `jmap submission get` and `query` for an id the JMAP server did not find |
| `permission-denied` | the server refused the item, or the command, to the account: a JMAP `forbidden` (SetError or method error), an HTTP 403 that is no rate limit (Gmail) or quota (Microsoft Graph), a JMAP HTTP 403, an IMAP `NOPERM` |
| `quota-exceeded` | the write would exceed the account's quota: a JMAP `overQuota`, an IMAP `OVERQUOTA`, Graph's `ErrorQuotaExceeded` or 507 whatever the status |
| `rate-limited` | the server still throttled the item, or the command, after the retries: an HTTP 429, a Gmail 403 `rateLimitExceeded`, `userRateLimitExceeded` or `quotaExceeded`, a JMAP `rateLimit` |
| `recipient-refused` | `message send` reached nobody because the server refused a recipient: an SMTP `RCPT TO` refused, naming the first refused address, or a JMAP `invalidRecipients`, naming the addresses it lists |
| `store-version` | a pimdir store is of another draft of the format; rebuild it as neverest's MIGRATION.md says, its queue drained first, or upgrade Himalaya |
| `store-uncreated` | a pimdir store has no schema yet, or its root holds none; its sync engine creates it |
| `store-foreign` | the database at a pimdir root is not a pimdir store; another program wrote it |
| `store-corrupt` | a pimdir store's version records disagree, or its schema is not its draft's |
| `store-not-current` | a pimdir store holds no schema yet and the verb creates none; its sync engine creates it |
| `store-hash-algo` | a pimdir store names its bodies with a hash Himalaya does not compute |
| `sync-version` | a pimdir store's sync state was written by another version of its sync engine |
| `store-busy` | another writer holds a pimdir store's write lock, or another process an mbox file's dotlock or fcntl lock past `mbox.lock.timeout`; retry |
| `store-owned` | a sync owns a pimdir store; retry once it ends |
| `store-staging` | a producer is staging a body in a pimdir store; retry |
| `store-in-flight` | a pimdir store verb is between two of its chunks; retry |
| `store-exposed` | a pimdir store is a link or another user can read or write it; restrict it |
| `store-reset` | a pimdir store is not the one that answered the queue row's handle, a copy restored since; nothing is done with the row |
| `lock-replaced` | a pimdir store's lock file was removed or replaced under a write; nothing is written; retry |
| `item-duplicate` | a pimdir mailbox holds the message already, or has its creation queued; nothing is staged |
| `flag-invalid` | a flag a pimdir store cannot hold: empty, or holding whitespace, a control or a non-ASCII character; nothing is staged |
| `action-malformed` | a pimdir action's payload breaks its schema; nothing is staged |
| `conflict-unread` | a pimdir conflict holds a body nobody has read; fetch it first |
| `unsupported` | a source of the pimdir store does not support the write; nothing is staged |
| `no-performer` | no source of the pimdir store can perform the intent |
| `ambiguous-performer` | several sources of the pimdir store can perform the intent and none is chosen |
| `index-missing` | a pimdir store has no search index yet; its sync engine builds it |
| `index-outdated` | a pimdir search index was built by another version; its sync engine rebuilds it |
| `index-version` | a pimdir search index is of another draft or shape; its sync engine rebuilds one of an earlier draft |
| `index-zone-unknown` | the zone a pimdir search index reads floating times in is no IANA zone |
| `index-fts5-missing` | the SQLite Himalaya links lacks what a pimdir search index needs (FTS5, 3.43 or later) |
| `query-refused` | a pimdir search query breaks a SEARCH §8.4 rule; nothing is searched |
| `query-unanswerable` | a pimdir search index could not answer an accepted query, an implementation fault |
| `tag-refused` | a pimdir tag names no flag, or adds and removes one flag; nothing is staged |
| `store-account-ambiguous` | several accounts of a pimdir store hold mail and `pimdir.account` names none; nothing is read |
| `store-account-unknown` | a pimdir store holds no collection of the account `pimdir.account` names; nothing is read |
| `row-not-found` | `pimdir queue cancel` names a row the account's queue does not hold; nothing is cancelled |
| `confirmation-required` | a command asks to confirm (`pimdir queue cancel`) with no terminal to answer and no `--yes`; nothing is done |
| `page-token-invalid` | `pimdir search` or `pimdir attachment list|search` is given a page token it did not print; nothing is listed |

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`


### Requirement: pimdir searches through the store's index
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-draft-13 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. The answer SHALL be the store as of the owner's last refresh: a queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort date|name|relevance` orders the hits (date by default), `--page` and `--page-size` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date, and its hits. Under `--json` the page SHALL print `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`: `nextPage` the number of the next page, as a string, while hits or threads follow the page, `null` once the answer is complete, which `--page-token` SHALL take back, winning over `--page`, a token that is no page number from 1 being refused, coded `page-token-invalid`. A hit whose message the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other hits listed (a thread without that hit) and the exit status 3; in text the entries print after the items, as every list prints them.

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

