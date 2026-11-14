---
cairn: delta
change: a-result-carries-its-errors
---

# Delta

## ADDED Requirements

### Requirement: A command acting on several items lists what failed
(commands) `envelope list`, `envelope search`, `mailbox list`, `flag add|set|remove`, `message copy`, `message move` and `message delete` SHALL print an `errors` list beside their result, under `--json` and in text: one pimalaya-cli item error `{id, code, error, sources}` per item the command could not list or act on, `[]` when there is none, in the order the items were met. `id` SHALL be the item's id as the command prints ids (an envelope id, a mailbox id), `null` when the backend's answer named none; `code` a code of the vocabulary or `null`; `sources` the backend's own reasons, outermost first. The items listed or acted on SHALL print as when nothing failed, `count` counting those alone. A non-empty `errors` SHALL exit 3. A failure leaving nothing attempted (the connection, the authentication, a mailbox not found, a refused query, a missing capability) SHALL stay the failure object, exit 1. The commands acting on one item (`message read`, `message add`, `message send`, the composers, `mailbox create|rename|delete`, `attachment list|download`) and the protocol namespaces SHALL print no `errors`.

#### Scenario: A listing with one unreadable message
- GIVEN a Maildir holding three messages, one of whose files cannot be read
- WHEN `himalaya --json envelope list` runs
- THEN it prints the two other envelopes and `"errors":[{"id":"<its id>","code":"message-unreadable",…}]`, and the exit status is 3

### Requirement: An item that failed prints in the header block
(commands) In the text output, each `errors` entry of a list SHALL print as one `Error:` field of the block above the items, after the paging fields: the id, `-` when null, the code, `-` when null, then the message, two spaces apart; nothing SHALL print under the items. A write SHALL print its sentence, then one `Error:` line per entry.

#### Scenario: One unknown id of a write
- GIVEN a mailbox holding message `42` and no message `99`
- WHEN `himalaya flag add --flag flagged 42 99` runs
- THEN it prints `Successfully added flags to 1 message(s): \Flagged`, then `Error:  99  message-not-found  …`, and the exit status is 3

### Requirement: Each backend names the items it could not serve
(backends) A backend SHALL report an item it could not list or act on as an `errors` entry, as precisely as its protocol tells, and SHALL never drop it with a warning alone.

- IMAP: a FETCH line io-imap recovered in part SHALL list its message with the items that decoded and add a `message-unreadable` entry with its UID naming the failed items, `hasAttachment` `null` when `BODYSTRUCTURE` is one; a line io-imap skipped whole SHALL add an entry with a `null` id. A message a listing's window did not return SHALL be no error, the mailbox having changed since its `SELECT`. A UID given to a write and absent from both the `UID STORE` echoes and a `UID SEARCH` on the set SHALL be `message-not-found`. A refused `COPY` or `MOVE`, which answers for the whole set, SHALL stay a whole failure coded by its response code.
- JMAP: an object that did not deserialize SHALL be `message-unreadable`. An id `Email/set` or `Email/copy` names in `notUpdated`, `notDestroyed` or `notCreated` SHALL take the code of its SetError type (`notFound` `message-not-found`, `forbidden` `permission-denied`, `overQuota` `quota-exceeded`, `rateLimit` `rate-limited`, any other `null`, its type in `sources`). An id the chained `Email/get` reports in `notFound` SHALL be left out of a listing.
- Gmail: a listing SHALL read its metadata through batched `messages.get`; a part answered 404 SHALL be left out, a part still throttled after the retries SHALL be `rate-limited`, another 403 `permission-denied`, an unanswered part an entry coded `null`. A write SHALL look its ids up through batched `messages.get`, an id Gmail does not know or whose message lacks the mailbox's label being `message-not-found`, and SHALL carry on past a failed message.
- Microsoft Graph: a write SHALL look its ids up and act on them through `$batch`; a part answered 400 or 404 SHALL be `message-not-found`, 403 `permission-denied`, 429 after the retries `rate-limited`, any other `null` with Graph's `error.code` in `sources`; a part the answer left out SHALL be an entry. A listing, one request, SHALL fail whole.
- Maildir: a file that cannot be read SHALL be `message-unreadable` with its id, a folder that cannot be read `mailbox-unreadable` with its name.
- Maildir, mbox, pimdir: an unknown id SHALL be `message-not-found`. An mbox write, one rewrite under the lock, and a pimdir write, one queue transaction, SHALL stay whole failures. io-pimdir's write notes SHALL stay warnings.
- A mailbox whose counts could not be fetched (`--counts`) SHALL be `mailbox-unreadable`, its counts `null`.

A `message send` whose server refused a recipient (an SMTP `RCPT TO` refused, a JMAP `invalidRecipients`) SHALL fail whole, sent to nobody, coded `recipient-refused`, naming the refused addresses.

#### Scenario: A Graph write with one part refused
- GIVEN a Graph folder holding messages `a` and `b`, the `$batch` answering the `PATCH` of `a` 200 and of `b` 403
- WHEN `himalaya --json flag add --flag seen a b` runs
- THEN it prints `count` 1 and `"errors":[{"id":"b","code":"permission-denied",…}]`, and the exit status is 3

## MODIFIED Requirements

### Requirement: A command given message ids says how many it found
(commands) `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL require ids, a repeated id being acted on and counted once. An id the mailbox does not hold SHALL be an `errors` entry coded `message-not-found`, the others acted on; when none exists, nothing SHALL change and the command SHALL print its result with `count` 0 and one entry per id, exiting 3. They SHALL report how many were acted on as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `raw` spelled as the backend writes it (`$flagged` on JMAP), `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `pimdir flag add|set|remove` SHALL fail, exit code 1, with the code `message-not-found` when none of the given ids exists, changing nothing. `message delete` in the trash SHALL count the ids the trash holds and remove those alone. On a backend whose ids are account-wide (JMAP, Gmail, Microsoft Graph), an id SHALL count as found only when its message is in the given mailbox, and a move whose source and destination are one mailbox SHALL change nothing there and on Maildir. On Maildir and mbox every id SHALL be looked up before the first write, an unknown one an entry, a removal from the trash included. The protocol commands of the local stores taking ids (`maildir` and `mbox` `message copy|move`, `maildir` and `mbox` `flag add|set|remove`) SHALL require them too and look every id up before the first write, an unknown one refusing the whole set and nothing changed: naming every unknown id under `message-not-found`; the flag commands SHALL print each id once. A `maildir message move` whose source and target are one Maildir, without `--subdir`, SHALL change nothing, the ids kept. `maildir message copy|move` and `mbox message copy|move` SHALL print the ids the messages have in the target, each message once. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya --json flag add --flag flagged 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":0,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

### Requirement: The exit status tells the outcome
(commands) A command SHALL exit 0 when it succeeds and 1 when it prints the failure object. A command line the grammar refuses (an unknown flag or command, a missing argument, a bad value) SHALL exit 2, parsed through pimalaya-cli's `clap::parse`: with a `--json` word before any `--`, it SHALL print on stdout the failure object coded `usage-invalid`, its `error` clap's message without its `error: ` prefix or styles, and clap's rendering on stderr otherwise. A command that printed its result without fully succeeding (a refused protocol completion, an `account check` with a failed backend, a result whose `errors` is not empty) SHALL exit 3, printing nothing after its result. Help and version SHALL print and exit 0 as clap does.

#### Scenario: A mistyped flag under --json
- WHEN `himalaya --json envelope list --nope` runs
- THEN stdout holds the failure object `{"code":"usage-invalid","error":"unexpected argument '--nope' found…","sources":[],"backtrace":null}`, stderr is empty, and the exit status is 2

#### Scenario: A failed check
- GIVEN an account whose Maildir root does not exist
- WHEN `himalaya --json account check` runs
- THEN it prints the report, the `maildir` row `ok: false`, and the exit status is 3

### Requirement: The JSON contract is described and versioned
(commands) `json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary and pimalaya-cli's `usage-invalid`, the item error of an `errors` list under `himalaya-item-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.


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
| `mailbox-not-found` | the mailbox named does not exist (Maildir, mbox, IMAP); nothing is done |
| `message-not-found` | a message id given is not in the mailbox: an `errors` entry beside what was done; for the raw `maildir` and `mbox` commands, one of them is not, or the one message read is not (IMAP, mbox), and nothing is done |
| `message-unreadable` | the backend answered for a message the client could not read: its FETCH line or one of its items did not decode (IMAP), its object did not deserialize (JMAP), its file could not be read (Maildir); an `errors` entry, the message listed with what was read when there is any, or the failure of a `message read` |
| `mailbox-unreadable` | a mailbox was listed but could not be read: its LIST or STATUS line did not decode (IMAP), its folder could not be read (Maildir), its counts could not be fetched; an `errors` entry |
| `permission-denied` | the server refused the item, or the command, to the account: a JMAP `forbidden`, an HTTP 403 that is no rate limit (Gmail, Microsoft Graph), an IMAP `NOPERM` |
| `quota-exceeded` | the write would exceed the account's quota: a JMAP `overQuota`, an IMAP `OVERQUOTA`, Graph's `ErrorQuotaExceeded` |
| `rate-limited` | the server still throttled the item after the retries: an HTTP 429, a Gmail 403 `rateLimitExceeded` or `userRateLimitExceeded`, a JMAP `rateLimit` |
| `recipient-refused` | `message send` reached nobody because the server refused a recipient: an SMTP `RCPT TO` refused, a JMAP `invalidRecipients`; the error names the refused addresses |
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

### Requirement: An IMAP answer missing lines says so
(backends) An untagged line io-imap could not decode SHALL never be dropped silently, in the unified commands and the `imap` ones alike. In a unified command printing `errors`, a FETCH line io-imap recovered in part SHALL list its message with the items that decoded and add a `message-unreadable` entry with its UID naming the items that did not, `hasAttachment` `null` when `BODYSTRUCTURE` is among them; a line io-imap skipped whole SHALL add an entry with a `null` id, `message-unreadable` for a FETCH line and `mailbox-unreadable` for a LIST line, and a STATUS line that did not decode SHALL be `mailbox-unreadable` naming its mailbox, its counts `null`. The `imap` commands SHALL take such lines with a warning counting them and naming the command. A `message read` whose `FETCH` answer held such a line and no message SHALL fail coded `message-unreadable`, saying the answer does not decode, never as `message-not-found`. The text of a server `ALERT` SHALL be logged as a warning.

#### Scenario: A listing with a line that does not decode
- GIVEN a mailbox one of whose messages nests seven `multipart` levels, its `BODYSTRUCTURE` refused by imap-codec
- WHEN `himalaya --json envelope list --has-attachment` reads it
- THEN the other envelopes print, that message prints with `hasAttachment: null`, an `errors` entry coded `message-unreadable` names its UID and `BODYSTRUCTURE`, and the exit status is 3

### Requirement: A tool's output is the command's payload
(mcp) A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is typed `object` and the tool has one output shape, save `attachment_download`, whose bytes travel as an embedded resource instead. The events a command prints SHALL come first in `content`, one text item each, the result last. A command ending with a partial outcome (a refused IMAP, SMTP or ManageSieve completion, a failed `account check`, a result whose `errors` is not empty) SHALL keep its result as `structuredContent` and set `isError: true`. A refused or failed call, a command printing no value or several (`The command printed <n> values, one expected`) and a part tool whose command printed no part (`The command printed no part to embed`) included, SHALL return `isError: true` with the `himalaya-error` object and no `structuredContent`; an unknown tool SHALL be a JSON-RPC error.

## REMOVED Requirements

None.
