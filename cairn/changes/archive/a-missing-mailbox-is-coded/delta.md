---
cairn: delta
change: a-missing-mailbox-is-coded
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: A server's refusal codes the whole command
A command failing whole because a server refused it SHALL carry the code of that refusal, read from the typed error of the protocol that answered it anywhere in the error chain: an IMAP completion naming `NOPERM` `permission-denied` and one naming `OVERQUOTA` `quota-exceeded` (RFC 5530 §3); a Gmail or Microsoft Graph answer as the same answer codes an item; a JMAP HTTP 429 `rate-limited` and 403 `permission-denied`, whichever request it answered, a method error `forbidden` `permission-denied`, and the SetError refusing the one write of a command by its type (`forbidden`, `overQuota`, `rateLimit`). The code SHALL be given at every exit: a one-shot run, a REPL line, an MCP call, and the row of an `account check`. Inside a protocol namespace the refusal SHALL be the command's result rather than a failure (*A protocol refusal is a result*), its `refusal.code` carrying that code. A command reading one message (`message read`, `attachment list`, `attachment download`, the source of `message reply` and `message forward`) whose id the backend does not hold SHALL fail coded `message-not-found` on every backend, a server's answer kept in `sources`. A command addressing a mailbox the backend does not hold SHALL fail coded `mailbox-not-found` on every backend, the copy a send saves an `errors` entry so coded: on IMAP a `SELECT`, `APPEND`, `COPY`, `MOVE`, `RENAME` or `DELETE` the server refuses `NO [NONEXISTENT]` (RFC 5530 §3) or `NO [TRYCREATE]` (RFC 3501 §6.3.11), or refuses with a plain `NO` for a mailbox the one `LIST` then sent does not show, the server's answer kept in `sources`; no `LIST` SHALL be sent before such a command, nor after a success, a `BAD` or a `NO` with another code, which keep the server's refusal (but after a `DELETE`, whose refusal names the mailboxes the one deleted holds); a mailbox `mailbox list` lists whose `STATUS` is refused `NONEXISTENT` is an `errors` entry so coded; on JMAP, Gmail and Microsoft Graph a name, id or role no mailbox, label or folder carries (Graph's lookup of an id answered 404 or `ErrorInvalidIdMalformed` included), refused before any write.

#### Scenario: A mailbox the server answers a plain NO for
- GIVEN an IMAP server answering `SELECT Nope`, `APPEND Nope` and `UID MOVE 5 Nope` with a plain `NO Mailbox does not exist`, and listing no `Nope`
- WHEN `himalaya --json envelope list -m Nope`, `message add -m Nope <message>` or `message move -t Nope 5` runs
- THEN each exits 1 with the failure object coded `mailbox-not-found`, the server's `NO` in `sources`, and `himalaya --json message send --save Nope <message>` prints `sent: true` beside an `errors` entry so coded, exit 3

#### Scenario: A mailbox the server forbids
- GIVEN an IMAP server answering `CREATE` with `NO [NOPERM]`
- WHEN `himalaya --json mailbox create Denied` runs, or the line `mailbox create Denied` in a `repl`, or the MCP call `mailbox_create`
- THEN each answers the failure object coded `permission-denied`

#### Scenario: A message the mailbox does not hold
- GIVEN a Maildir account
- WHEN `himalaya --json message read 999` runs
- THEN it exits 1 and prints the failure object coded `message-not-found`, naming `999` and the mailbox

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
| `mailbox-not-found` | the mailbox named does not exist (Maildir, mbox, IMAP, JMAP, Gmail, Microsoft Graph, pimdir); nothing is done, or, for the copy a send saves, an `errors` entry beside the receipt; an `errors` entry of an IMAP `mailbox list` whose `STATUS` is refused `NONEXISTENT`; an `errors` entry of `jmap mailbox get` and `query` for an id the JMAP server did not find |
| `mailbox-duplicate` | a create names a mailbox that exists: a Maildir folder or an mbox file already there, a mailbox a pimdir store lists or has its creation queued, an IMAP `CREATE` refused `ALREADYEXISTS` or refused with no code while `LIST` shows the mailbox, a JMAP `Mailbox/set` refused while a sibling holds the name, a Gmail 409, a Graph 409 `ErrorFolderExists` or a child of that name under the parent; nothing is created nor staged |
| `message-not-found` | a message id given is not in the mailbox: an `errors` entry beside what was done; for the raw `maildir` and `mbox` commands, one of them is not, or the one message a command reads is not (every backend), and nothing is done; an `errors` entry of `jmap email get` and `query` for an id the JMAP server did not find |
| `message-unreadable` | the backend answered for a message the client could not read: its FETCH line or one of its items did not decode (IMAP), its object did not deserialize (JMAP), its file could not be read (Maildir), it holds no header field (Maildir, mbox), the store could not read its row (`pimdir search`); an `errors` entry, the message listed with what was read when there is any, or the failure of a `message read` |
| `mailbox-unreadable` | a mailbox was listed but could not be read: its LIST or STATUS line did not decode (IMAP), its folder could not be read (Maildir), its counts could not be fetched; an `errors` entry |
| `thread-not-found` | an `errors` entry of `jmap thread get` for an id the JMAP server did not find |
| `identity-not-found` | an `errors` entry of `jmap identity get` for an id the JMAP server did not find |
| `submission-not-found` | an `errors` entry of `jmap submission get` and `query` for an id the JMAP server did not find |
| `permission-denied` | the server refused the item, or the command, to the account: a JMAP `forbidden` (SetError or method error), an HTTP 403 that is no rate limit nor daily quota (Gmail) or quota (Microsoft Graph), a JMAP HTTP 403, an IMAP `NOPERM` |
| `quota-exceeded` | the write would exceed the account's quota: a JMAP `overQuota`, an IMAP `OVERQUOTA`, Graph's `ErrorQuotaExceeded` or 507 whatever the status |
| `rate-limited` | the server still throttled the item, or the command, after the retries: an HTTP 429, a Gmail 403 `rateLimitExceeded`, `userRateLimitExceeded`, `quotaExceeded` or `dailyLimitExceeded` (or a message naming a daily limit or a quota per day, the unauthenticated one aside), as the family codes a Google quota, a JMAP `rateLimit` |
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

### Requirement: Graph folders are named by path
Graph SHALL list every mail folder but the hidden ones, walking the child folders of each folder holding some and following every paging link, a nested folder named by its path, the display names joined with `/` (`Projets/Alpha`). A mailbox value matching no listed id, path or role SHALL be taken, when it is a well-known folder name carrying a role (compared ignoring case), as the folder carrying that role (`deleteditems` the trash); else, when it is a well-known name or shaped like a folder id (32 base64url characters or more), it SHALL be looked up with `GET /mailFolders/{value}`, which reaches a hidden folder. A value Graph answers 404 for, or 400 with the `error.code` `ErrorInvalidIdMalformed`, or any other value, SHALL be refused with ``Mailbox `X` not found``, coded `mailbox-not-found`, never sent into a request path; another failure of that lookup, another 400 included, SHALL fail the command. io-msgraph SHALL percent-encode every id it puts in a request path as one segment.

#### Scenario: A nested folder
- GIVEN a Graph account holding the folder `Alpha` under `Projets`
- WHEN `mailbox list` runs, then `envelope list -m Projets/Alpha`
- THEN the folder is listed as `Projets/Alpha`, and its envelopes are listed

## REMOVED Requirements
