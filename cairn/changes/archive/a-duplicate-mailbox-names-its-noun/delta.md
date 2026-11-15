---
cairn: delta
change: a-duplicate-mailbox-names-its-noun
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Unified mailbox writes
`mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST "" ""` reports, `/` on Gmail, Graph and the local stores. NAME SHALL be refused when empty, blank or holding a control character, and per backend when it holds the hierarchy delimiter (IMAP's, `/` on Gmail, Maildir and the unified mbox writes), or names no listable level (Maildir `cur`, `new`, `tmp`, a leading dot on Maildir, a dot on Maildir++, the names the mbox backend refuses). Creating an existing mailbox and renaming onto one SHALL be refused. A create naming a mailbox that exists SHALL fail coded `mailbox-duplicate`, exiting 1 with nothing created nor staged, wherever the backend can tell: a Maildir folder already at the path (a file there being no mailbox, refused uncoded), an mbox file already there (a directory refused uncoded), a mailbox a pimdir store lists or has its creation queued (backends, "pimdir creates a mailbox by queueing an intent"), an IMAP `CREATE` the server refuses with `ALREADYEXISTS` (RFC 5530 section 3), or refuses with no code while `LIST` then shows the mailbox, a JMAP `Mailbox/set` refused while a mailbox of that name sits under the same parent (siblings sharing no name, RFC 8621 section 2) and coded no other way, a Gmail label create answered 409, and a Graph folder Graph answers exists (409 `ErrorFolderExists`) at the top, or one of that name the parent holds, hidden ones included, looked up before the folder is created at the top. A Graph namesake at the top of a folder asked under a parent is no duplicate, its refusal kept uncoded. The protocol namespaces' own creates (`imap mailbox create`, `jmap mailbox create`, `gmail labels create`, `msgraph mail-folders create`) keep the server's refusal as it is; `maildir create` and `mbox create` code the duplicate as the unified create does. The root of a local store (the root Maildir, however named, and the mbox spool) SHALL never be deleted or renamed. Deleting a local mailbox that holds others SHALL be refused, naming them, and a Maildir holding any directory besides `cur`, `new` and `tmp`, listed or not, SHALL be refused too, naming it, the removal taking it along otherwise; on IMAP a server's refusal of the same SHALL name them too, and a missing mailbox SHALL read ``IMAP mailbox `X` not found`` for rename and delete alike. On IMAP a `--parent` the server marks `\Noinferiors` (RFC 3501 section 7.2.2) SHALL be refused before the `CREATE`, ``IMAP mailbox `X` can hold no other (\Noinferiors)``. On Gmail, where a label nests by name, a rename SHALL also rename every sublabel, as IMAP `RENAME` moves the inferiors; a delete SHALL leave the sublabels under their full names.

The outputs SHALL be `{id, name}` for create and rename, `id` being the mailbox's id afterwards (a rename changes it where the name is the id) and `null` when the write is queued, and `{id}` for delete, the id `mailbox list` prints. `name` SHALL be the full name `mailbox list` prints on IMAP, Maildir and mbox, and NAME as given on the other backends. A write SHALL drop the mailbox index a backend caches, so a later command of the session resolves the new name.

#### Scenario: A Maildir mailbox under a parent
- GIVEN a Maildir account holding `Archive`
- WHEN `mailbox create 2025 --parent Archive`, then `mailbox rename Archive/2025 2026`, then `mailbox delete Archive/2026` run
- THEN `Archive/2025` is created, renamed to `Archive/2026`, then removed

#### Scenario: The root Maildir
- GIVEN a Maildir account whose root is a Maildir holding `Archive`
- WHEN `mailbox delete .`, `mailbox delete INBOX` or `mailbox rename INBOX X` runs
- THEN it is refused, naming the store root, and the store is whole

#### Scenario: A mailbox created twice
- GIVEN a Maildir account holding the folder `Archive` and an mbox account holding the file `Archive`
- WHEN `himalaya --json mailbox create Archive`, `maildir create Archive` or `mbox create Archive` runs on either
- THEN it fails coded `mailbox-duplicate`, exiting 1, and the folder and the file are as they were

### Requirement: The raw Maildir folder commands guard the store
`maildir create` SHALL refuse every level of NAME the unified mailbox writes refuse (blank, holding a control character, unlistable), and an existing folder, coded `mailbox-duplicate`; `maildir rename` SHALL refuse the same of its NAME. `maildir rename` SHALL refuse the root, an existing target, and on the fs layout a target whose parent does not exist. `maildir delete` SHALL refuse the root, and removes the folder with every folder below it, nested Maildirs and Maildir++ `.<name>.*` folders included. Each SHALL print the name relative to the root. `maildir flag set` and `mbox flag set` SHALL require at least one `-f`, refusing the command line (exit 2) rather than clear every flag.

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary and pimalaya-cli's `usage-invalid`, the item error of an `errors` list under `himalaya-item-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.


| Code | Raised when |
|---|---|
| `usage-invalid` | a command line, a REPL line or an MCP tool's arguments the grammar refuses (an unknown flag, command or argument, a missing argument, a bad value); raised by pimalaya-cli beside Himalaya's codes; nothing runs |
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |
| `message-too-complex` | a message is nested deeper than 8 levels, or holds more than 200 parts or 500 header lines; it is refused whole |
| `stdin-taken` | a command inside a `repl` session would read its input from stdin, which carries the session's commands; pass it as an argument or a file path |
| `attachment-too-large` | an attachment is larger than `himalaya mcp --max-attachment-size`; one given inline is refused before it is decoded, one a tool reads before it is encoded; nothing is done, nothing of it returned |
| `attachment-invalid` | an `attach-data` item given to an MCP composer is malformed (no file name, both or neither of `data` and `text`, bad base64, a bad `mimeType`); nothing is composed |
| `search-invalid` | an `envelope search` query does not parse; nothing is searched |
| `search-unsupported` | an `envelope search` clause the account's backend cannot search, quoted as written; nothing is searched, no clause dropped |
| `capability-missing` | the account's backend does not serve the verb, or the option of it asked for, as its capability table declares or its server advertises; nothing is done, the message naming the capability and what serves it instead |
| `mailbox-not-found` | the mailbox named does not exist (Maildir, mbox, IMAP); nothing is done; an `errors` entry of `jmap mailbox get` and `query` for an id the JMAP server did not find |
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
| `store-version` | a pimdir store is of another draft of the format; upgrade Himalaya, or reinstall the store as neverest's MIGRATION.md says |
| `store-uncreated` | a pimdir store has no schema yet; its sync engine creates it |
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

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"row": i64, "name": text, "parent": collection | null, "source": id}`, `parent` `null` at the top, never left out. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A mailbox the store lists already SHALL be refused with `mailbox-duplicate`, naming it, before anything is queued, as "Unified mailbox writes" refuses creating an existing mailbox: one the performer's source syncs under NAME at the top, or under the parent's name, a hierarchy delimiter (`/` or `.`, the store recording none) and NAME, names compared exactly; so SHALL a creation of NAME under that parent the same source has queued already. A name the store does not list yet is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `row` and `"source":"imap"`

#### Scenario: A mailbox at the top
- GIVEN a pimdir account whose `gmail` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN it prints `"parent":null` beside `row`, `name` and `source`

#### Scenario: A mailbox the store lists
- GIVEN a pimdir account whose `imap` source declares `collection.create` and syncs `imap/Projets`, named `Projets`
- WHEN `himalaya mailbox create Projets` or `himalaya pimdir mailbox create Projets` runs
- THEN the command fails with the code `mailbox-duplicate`, naming `imap/Projets`, and nothing is queued

## REMOVED Requirements
