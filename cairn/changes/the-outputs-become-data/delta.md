---
cairn: delta
change: the-outputs-become-data
---

Folded into cairn/spec/commands.md, cairn/spec/backends.md, cairn/spec/search.md and cairn/spec/mcp.md on 2026-11-03.

## ADDED Requirements

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A shared write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message send` and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, m2dir, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Absent is null
An output type of Himalaya's own SHALL print every key, `null` when the value is absent, so its schema marks every key required. A provider payload printed as received SHALL keep its own omissions.

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page comes back full, `null` otherwise, and SHALL take it back through `--page-token`, which wins over `--page`. A full last page leads to an empty one. `msgraph messages list` SHALL print the `$skip` of Graph's next link as `nextPage` and take it back through `--page-token`; a next link carrying no `$skip` (a `$search` page) SHALL be printed whole and refused by `--page-token`. The Gmail listings keep Gmail's page token.

#### Scenario: A second page
- GIVEN a mailbox holding two messages
- WHEN `himalaya --json envelope list -s 1` runs, then `himalaya --json envelope list -s 1 --page-token 2`
- THEN the first prints one envelope and `"nextPage":"2"`, the second the other envelope

### Requirement: An IMAP command prints its completion
Every `imap` command but `raw` SHALL print the tagged completion of its exchange: `status: "OK"` beside what the command understood of the server's answer, or `{status, text}` when the server answers `NO` or `BAD`, exiting 2 after it. A command getting no completion (a network, TLS or parse failure, a local error) SHALL fail with the failure object. `select` SHALL print the mailbox's state (`flags`, `exists`, `recent`, `unseen`, `permanentFlags`, `uidNext`, `uidValidity`, `highestModSeq`), `expunge` the expunged message numbers, `store` the FETCH data the server returns, `append` the `APPENDUID` it returns (UIDPLUS), `copy` and `move` the `COPYUID`, `null` and empty without UIDPLUS, the mailbox writes `{mailbox}` and `close` and `unselect` the status alone. `fetch` SHALL print envelope addresses as `{name, email}`, and `search` its ids as numbers, as `sort` does. io-imap returns a typed exchange's output whole, so these commands print no events.

#### Scenario: A mailbox that does not exist
- GIVEN an IMAP account
- WHEN `himalaya --json imap select Nope` runs
- THEN it prints `{"status":"NO","text":"…"}` with the server's text and exits 2

### Requirement: A local store flag prints its meaning beside its spelling
`maildir flag list` and `mbox flag list` SHALL print each flag as `{raw, iana}`, `raw` being the store's letter and `iana` the keyword it stands for, `null` for mbox's `O`, which has none; mbox rows SHALL also name the `header` carrying the letter.

## MODIFIED Requirements

### Requirement: ManageSieve commands

When built with the `sieve` feature, the protocol-specific API SHALL expose `sieve capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Every command SHALL print a dedicated serializable output type: `activate`, `rename` and `delete` the receipt `{id}` naming the script, `deactivate` `{active: null}`, `check` and `put` `{name, warnings}`, `warnings` being the text a server attaches to an accepted script, `null` when it sends none, and `name` `null` for a check. `put` and `check` SHALL accept script bytes from a file, inline arguments, or stdin.

The protocol SHALL live in io-managesieve, so this repository holds a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, exactly as the other protocol modules do.

### Requirement: Raw passthrough is byte-verbatim
The `imap raw` and `smtp raw` commands SHALL forward the command bytes to the server verbatim, resolving the argument through the shared `RawCommandArg` (positional or stdin read whole, refused when not valid UTF-8). It decodes literal `\r` / `\n` escapes into real CRLF so a shell-typed command survives intact. `imap raw` sends a batch of caller-tagged commands: it appends a trailing CRLF when missing and delegates tagging, framing and out-of-order completion tracking to io-imap. `smtp raw` stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects a multi-line batch, since the SMTP exchange reads exactly one reply. `sieve raw` sends one command line and reads the complete ManageSieve response; literal-bearing Sieve commands use their typed commands so the response reader remains synchronized.

The reply SHALL be printed split into what himalaya understands of it. `imap raw` SHALL print each untagged response as an `untagged` event `{kind, number, text}` (`* 3 EXISTS` being kind `EXISTS`, number 3), each continuation request as a `continuation` event `{text}`, and a response it cannot split as an `unparsed` event `{text}`, a literal kept in the response announcing it; then the result `{completions: [{tag, status, code, text}]}`. `sieve raw` SHALL print each data line as a `data` event `{text}`, then the completion `{status, code, text}`. `smtp raw` SHALL print `{code, text}`, the reply's lines joined, `code` `null` when the reply opens with none. A refused completion (an IMAP `NO` or `BAD`, a ManageSieve `NO` or `BYE`, an SMTP 4xx or 5xx) SHALL exit 2, its result printed. `imap raw` reads the whole reply before printing, so its events come before the result but not as they arrive, and io-imap decodes the reply lossily, so a non-UTF-8 byte comes back as U+FFFD.

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole, one row `{backend, ok, code, error}` per backend, `code` and `error` `null` when the backend answered. When any fails, it SHALL exit 2 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/.

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print `code: null`. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help, or the wizard `configure` describes), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.

| Code | Raised when |
|---|---|
| `body-pending` | a listed message's body is not local yet (pimdir); a sync brings it |
| `message-too-complex` | a message is nested deeper than 8 levels, or holds more than 200 parts or 500 header lines; it is refused whole |
| `stdin-taken` | a command inside a `repl` session would read its input from stdin, which carries the session's commands; pass it as an argument or a file path |
| `search-invalid` | an `envelope search` query does not parse; nothing is searched |
| `search-unsupported` | an `envelope search` clause the account's backend cannot search, quoted as written; nothing is searched, no clause dropped |
| `capability-missing` | the account's backend does not serve the verb, or the option of it asked for, as its capability table declares or its server advertises; nothing is done, the message naming the capability and what serves it instead |
| `store-version` | a pimdir store is of another draft of the format; upgrade Himalaya, or reinstall the store as neverest's MIGRATION.md says |
| `store-uncreated` | a pimdir store has no schema yet; its sync engine creates it |
| `store-foreign` | the database at a pimdir root is not a pimdir store; another program wrote it |
| `store-corrupt` | a pimdir store's version records disagree, or its schema is not its draft's |
| `store-not-current` | a pimdir store holds no schema yet and the verb creates none; its sync engine creates it |
| `store-hash-algo` | a pimdir store names its bodies with a hash Himalaya does not compute |
| `sync-version` | a pimdir store's sync state was written by another version of its sync engine |
| `store-busy` | another writer holds a pimdir store's write lock; retry |
| `store-owned` | a sync owns a pimdir store; retry once it ends |
| `store-staging` | a producer is staging a body in a pimdir store; retry |
| `store-in-flight` | a pimdir store verb is between two of its chunks; retry |
| `store-exposed` | a pimdir store is a link or another user can read or write it; restrict it |
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

### Requirement: Data commands serialize their data
Every command SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key; a sentence is the text form of that data, never a JSON field. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. Where the wire type is unsuitable (a recursive MIME tree, a type carrying no schema), the output type SHALL name its fields instead. `gmail threads get` is the exception for the MIME tree: `users.threads.get` has no raw format, so each message's `payload` SHALL be serialized when the requested format carries one. A Gmail settings write SHALL print the settings the server returns, in the shape of the matching `get`. A Gmail list SHALL print the provider's collection under its own key (`delegates`, `filter`, `forwardingAddresses`), and `gmail messages list` its `messages`.

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`, and `msgraph messages get --raw`. The byte writer SHALL write the bytes verbatim in text mode and print `{data, size, mime}`, `data` in base64, under `--json`, so no command writes raw bytes to a JSON stdout; given a path, it SHALL write the file and print `{path, size}`. `gmail attachments get`, `msgraph attachments get` and `jmap email export` SHALL print through it.

### Requirement: A read message is a designed view
`message read --json` SHALL print `{headers, text, html, parts}`, built from the raw message so it is the same on every backend, rather than the parser's own model. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset or `null`, `messageId` without angle brackets, `inReplyTo` and `references` as lists without angle brackets, as envelopes print them, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, a `format=flowed` part unwrapped (RFC 3676 §4.2), joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The message SHALL be parsed by mime-rs, whose structure is the one RFC 2046 delimits: a line opening with a boundary but carrying more than transport padding is no delimiter, an outer delimiter closes the parts nested in it, and a field whose name holds a blank (`Sub ject:`) is not the field it spells. The text output SHALL show the same headers, parts, sizes and names as the JSON view. `--raw` SHALL write the message bytes verbatim, and print `{data, size, mime}` under `--json`.

### Requirement: Every part has an id and a role
`parts` SHALL list every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`, the `id` being the one `attachment list` and `attachment download` take: the 1-based position of the part in the whole part list. A `multipart/*` SHALL be no part; `message/rfc822` or `Content-Disposition: attachment` SHALL be an `attachment`; a `text/plain` or `text/html` without a filename SHALL be a `body`; an `image/*` marked inline or carrying a `Content-ID` SHALL be `inline`; anything else SHALL be an `attachment`. `filename` SHALL be RFC 2231 and RFC 2047 decoded, `null` when the part names none; `size` SHALL count the part's bytes once its transfer encoding is undone, in its own charset; `contentId` SHALL carry no angle brackets; `method` SHALL be a `text/calendar` part's `method` parameter, uppercased. `attachment list --json` SHALL report the same ids, sizes, `contentId` and `role`, and `path`, where `attachment download` wrote the part, `null` until then.

### Requirement: A command given message ids says how many it found
`flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL fail, exit code 1, when none of the given ids exists in the mailbox, changing nothing. When some exist they SHALL succeed and report how many as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `message delete` on a message already in the trash is not covered. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya flag add --flag flagged 99999999` runs
- THEN the command exits 1 with `No flag added: no id matched in the mailbox`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 0 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false}`

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the receipt of a shared write SHALL carry `queued: true`, and its text form SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy`, `flag add|set|remove` and `mailbox create|rename|delete`. A mailbox write SHALL print its `id` `null` while queued, and no queue detail (a row, a queue id) SHALL reach a shared output. Every other backend SHALL print `queued: false`.

#### Scenario: A send on a pimdir account
- GIVEN a pimdir account with `mailbox.alias.sent` set
- WHEN `message send` runs
- THEN it prints `Message queued for sending`, not that the message was sent

#### Scenario: A delete on a pimdir account
- GIVEN a pimdir account with a trash mailbox
- WHEN `message delete 3` runs on another mailbox
- THEN it prints `Queued the move of 1 message(s) to the trash`

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the shared `message add` and `message send` do, and print the queue row it waits in with the message's bare `Message-ID`: `{"row": i64, "messageId": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyRow": i64` (a copy queued beside the send, when its performer declares no capabilities) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found: `{"messages": [{"id": string, "row": i64 | null}]}`, `null` for a message the change left as it was, for which nothing was staged. The shared commands SHALL keep printing no queue detail. `pimdir queue list` and `queue cancel` SHALL name a row `row` as well.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4): `{"row", "state": "pending" | "parked" | "applied" | "unknown", "collection", "kind", "attempts", "error", "appliedAt", "seq"}`, a key the state does not carry being `null`. A pending or parked row carries `collection`, `kind` and `attempts`, a parked one `error` too; an applied one carries `collection`, `appliedAt` and, for an add, `seq`, the public id of the message it created. A row neither queued nor in the store's receipts (kept seven days), or one anchored on another account's collection, SHALL read as `unknown`: cancelled, or applied long ago. A row the sync engine performed rather than applied, a message sent or a mailbox created, renamed or deleted, reads as `applied` without `seq`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5). A receipt lives under the collection its row was queued on and follows it: a renamed mailbox's row reads `applied` under the mailbox's new id, and a deleted mailbox's row reads `unknown` once the deletion leaves the store, the receipt going with the collection, the mailbox missing from `pimdir mailbox list` being what says it was deleted.

The plain output of every `pimdir` subcommand SHALL end with a newline.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"row":12,"messageId":"d1@x.org"}`, and `pimdir queue show 12` prints `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `seq` the message is read by

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"row": i64, "name": text, "parent": collection | null, "source": id}`. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused, its owner predating the intent. An empty name, or one holding a control character, SHALL be refused before anything is staged. A name the server already holds is the server's to refuse: the row parks, and `pimdir queue show` says why.

#### Scenario: A folder is created through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.create`
- WHEN `himalaya --json pimdir mailbox create Projets` runs
- THEN one `collection-create` row is queued with `{"v":1,"source":"imap","name":"Projets"}`, and the command prints its `row` and `"source":"imap"`

### Requirement: pimdir renames or deletes a mailbox by queueing an intent
`pimdir mailbox rename <MAILBOX> <NAME>` SHALL enqueue one `collection-rename` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text}` and print `{"row": i64, "mailbox": collection, "name": text, "source": id}`. `pimdir mailbox delete <MAILBOX>` SHALL enqueue one `collection-delete` intent with `{"v": 1, "source": id}` and print `{"row": i64, "mailbox": collection, "source": id}`. Neither SHALL rename or delete a collection in the store: the change arrives with the sync that performs the intent.

The row SHALL be anchored on the mailbox, which must be a mailbox of the account (an id or a role, after the account's aliases), the payload never restating it, and queued through io-pimdir's `enqueue_collection_rename` or `enqueue_collection_delete`. The performer SHALL be named as for a creation (capability `collection.rename` or `collection.delete`, each gating its own intent): several candidates and no recorded choice are refused, naming them, and a source declaring the capability `none` on that mailbox (an IMAP `INBOX`, a system folder) refuses it. A store whose sources declare no capabilities SHALL be refused. A rename name empty or holding a control character SHALL be refused before anything is staged.

A rename keeps the mailbox's parent, `name` being one level of the hierarchy. The store records no hierarchy separator, so a name holding the server's, or one a sibling already holds, is the performer's to refuse: the row parks, and `pimdir queue show` says why. A delete removes the mailbox and the messages it holds on the server, and never another mailbox: the store records no mailbox's parent, so a mailbox holding others is the performer's to refuse, the row parking naming them.

#### Scenario: A folder is renamed through the sync engine
- GIVEN a pimdir account whose `imap` source declares `collection.rename`
- WHEN `himalaya --json pimdir mailbox rename imap/Projets Archives` runs
- THEN one `collection-rename` row is queued on `imap/Projets` with `{"v":1,"source":"imap","name":"Archives"}`, and the command prints its `row`, `"mailbox":"imap/Projets"` and `"source":"imap"`

#### Scenario: A folder holding another is not deleted
- GIVEN a pimdir account whose `imap` source declares `collection.delete`, and a server mailbox `Projets` holding `Projets/2025`
- WHEN `himalaya pimdir mailbox delete imap/Projets` runs and the sync engine performs the row
- THEN the server deletes nothing, the row parks naming `Projets/2025`, and `pimdir queue show` prints it as `parked` with that error

### Requirement: pimdir stages flags by their raw spelling
`himalaya pimdir flag list|add|set|remove` SHALL read and stage the flags of a pimdir message by their raw spelling, verbatim. A value naming an IANA flag SHALL be accepted only in its canonical spelling, compared without case and staged in the canonical case, a bare `seen` or `$seen` being refused naming `\Seen`; `\Deleted` and `\Recent` SHALL be refused. A backslash names a system flag, so one in front of any other word SHALL be refused, as SHALL a character IMAP forbids in a flag (`(`, `)`, `{`, `%`, `*`, `"`, `]`, `\`). A value the store cannot hold (pimdir STORAGE §13), empty or holding whitespace, a control character or a character outside ASCII, SHALL be refused with the code `flag-invalid`, the code io-pimdir's enqueue gives a flag set holding one, and nothing staged. Any other value is a keyword.

`add`, `set` and `remove` SHALL fail when no message is named or none of the named ones exists, and SHALL stage one `SetFlags` per message through the backend's `stage_flags`, checked against the sources' declarations (pimdir STORAGE §15.6): a source declaring `mail.flags.keywords` as `none` SHALL refuse a keyword change with its detail, and one declaring it in part SHALL queue it with the declaration's warning as it is. A change on `$Important` or `$Junk` SHALL be gated by `mail.flags.important` or `mail.flags.junk` from a source stating that row, `none` included, and by `mail.flags.keywords` from one stating none (pimdir STORAGE Annex B.1), io-pimdir's gate deciding which. `list` SHALL print the flags a message carries as the overlaying reader shows them, and `null` under `--json` for a message whose flags the sync engine never read.

An add or a remove SHALL match the flags a message carries as flags, not as spellings: a marker the store holds as `\seen`, or as Thunderbird's `Junk`, is the one `\Seen` or `$Junk` keeps on an add and drops on a remove. This holds for the shared flag commands on a pimdir account, which stage through the same path.

#### Scenario: A keyword is staged and read back
- GIVEN a pimdir account whose `imap` source declares `mail.flags.keywords`
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f act 42` runs
- THEN it prints `{"messages":[{"id":"42","row":7}]}`, and `pimdir flag list -m imap/INBOX 42` lists `act` before the next sync

#### Scenario: A source without keywords refuses one
- GIVEN a pimdir account whose `msgraph` source declares `mail.flags.keywords` as `none`
- WHEN `himalaya pimdir flag add -m msgraph/Inbox -f act 42` runs
- THEN the command fails naming `mail.flags.keywords`, the `msgraph` source and its detail, and nothing is queued

#### Scenario: A flag held in another spelling is removed
- GIVEN a pimdir message carrying `\seen` and `Junk`
- WHEN `himalaya pimdir flag remove -m imap/INBOX -f '\Seen' -f '$Junk' 42` runs
- THEN the staged set holds neither

#### Scenario: A bare IANA name is refused
- WHEN `himalaya pimdir flag add -m imap/INBOX -f seen 42` runs
- THEN the command fails naming `\Seen`, and nothing is queued

#### Scenario: A flag the store cannot hold
- GIVEN any pimdir account
- WHEN `himalaya --json pimdir flag add -m imap/INBOX -f 'to do' 42` runs
- THEN the command exits 1 with the code `flag-invalid`, and nothing is queued

### Requirement: pimdir counts, threads and tags from the index
`himalaya pimdir count <query>...` SHALL answer how many messages the scoped query matches, with its coverage. `himalaya pimdir thread <id>` SHALL list the thread of one message, oldest first, a message no thread holds being its own. `himalaya pimdir tag <query>... --add <flag> --remove <flag>` SHALL stage one `set-flags` per placement each hit's flags change in, through the queue as a producer (io-pimdir `PimdirSearchTagPlan::stage`), every action passing the capability gate before the first is appended, and SHALL report each placement as `queued` with its row, `unchanged`, `unknown-flags` (its flags were never read), `ambiguous-source`, `unsynced` or `unsupported`, staging nothing for those. `read`, `flagged`, `answered` and `draft` name `\Seen`, `\Flagged`, `\Answered` and `\Draft` as `is:` does; a flag is otherwise checked as `pimdir flag` checks it. `--dry-run` reports the plan and stages nothing. Himalaya SHALL NOT drain the queue: the owner applies the rows.

#### Scenario: Tagging a query
- GIVEN two messages of the index from `alice@x.org`, one already flagged
- WHEN `himalaya pimdir tag from:alice@x.org --add flagged` runs
- THEN one `set-flags` row is queued for the other one, and the flagged one is reported unchanged

### Requirement: A tool's output is the command's payload
A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is an object and the tool has one output shape. The events a command prints SHALL come first in `content`, one text item each, the result last. A command ending with a partial outcome (a refused IMAP, SMTP or ManageSieve completion, a failed `account check`) SHALL keep its result as `structuredContent` and set `isError: true`. A refused or failed call SHALL return `isError: true` with the `himalaya-error` object; an unknown tool SHALL be a JSON-RPC error.
