---
cairn: delta
change: the-contract-audit-rulings
---

# Delta

## ADDED Requirements

### Requirement: A missing backend is named
When no configured backend matches `--backend`, the error SHALL name the account and, under `auto`, the backends the command supports in this build; under a named backend, the missing block, or that the backend is unsupported.

### Requirement: No m2dir store is read
Himalaya SHALL NOT read m2dir stores: an `m2dir` block is an unknown key (*Unknown keys are refused*), and `m2dir` SHALL NOT be a `--backend` value, clap refusing it as any unknown value.

## MODIFIED Requirements

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole as pimalaya-cli's `account::AccountCheckOutput`, `{accounts}`, a list holding the one account checked, `{account, backends}`, one row `{backend, ok, code, error, tls}` per backend, pimalaya-cli's `account::BackendCheck`, `code` and `error` `null` when the backend answered, a failure coded as a one-shot run codes it. `tls` SHALL name the TLS a connection over TLS (an `imaps`, `smtps` or `sieves` server, a STARTTLS upgrade, an `https` JMAP server, Gmail, Graph) was configured with, its own `tls` block over the account's (config, "One TLS policy per account"), as pimalaya-cli's `account::BackendCheckTls`, `{provider, crypto, cert, clientCert}`, each `null` where the build's default answers, and SHALL be `null` for a local backend and for a cleartext connection; the text is pimalaya-cli's, appending `TLS` and the configured parts to a row over TLS, `OK, TLS native-tls, cert /etc/ca.pem`, `OK, TLS` when the build's defaults answer. When any fails, it SHALL exit 3 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/. A Maildir block SHALL be checked by listing its store, a missing root failing the check.

### Requirement: A query error is drawn without colour
The report `envelope search` and the pimdir search commands draw over a query they refuse SHALL carry no colour escape. Under `--json` the error string SHALL be one line, no diagram drawn: `envelope search` prints the error, then for each fault `; at <start>..<end>, ` and what it found and expected, the span in bytes of the query.

### Requirement: The exit status tells the outcome
A command SHALL exit 0 when it succeeds and 1 when it prints the failure object. A command line the grammar refuses (an unknown flag or command, a missing argument, a bad value) SHALL exit 2, parsed through pimalaya-cli's `clap::parse`: with a `--json` word before any `--`, it SHALL print on stdout the failure object coded `usage-invalid`, its `error` clap's message without its `error: ` prefix or styles, and clap's rendering on stderr otherwise. A command that printed its result without fully succeeding (a refused protocol completion, an `account check` with a failed backend, a result whose `errors` is not empty) SHALL exit 3, printing nothing after its result. Help and version SHALL print and exit 0 as clap does. A bare `himalaya` SHALL print the help, or meet a person missing a configuration with the welcome, through pimalaya-cli's `clap::meet_bare_invocation`; under `--json` it SHALL fail coded `usage-invalid`, exit 2, a JSON reader never getting text. A command partly failing SHALL say so in its help: `account check`, `imap raw`, `flag add|set|remove`, `message copy|move|delete` and `envelope list|search` name their exit 3.

#### Scenario: A mistyped flag under --json
- WHEN `himalaya --json envelope list --nope` runs
- THEN stdout holds the failure object `{"code":"usage-invalid","error":"unexpected argument '--nope' found…","sources":[],"backtrace":null}`, stderr is empty, and the exit status is 2

#### Scenario: A failed check
- GIVEN an account whose Maildir root does not exist
- WHEN `himalaya --json account check` runs
- THEN it prints the report, the `maildir` row of its one account `ok: false`, and the exit status is 3

#### Scenario: A bare run under JSON
- WHEN `himalaya --json` runs with no command
- THEN stdout holds the failure object coded `usage-invalid` and the exit status is 2

### Requirement: The JSON contract is described and versioned
`json-schema` SHALL describe the failure object under the key `himalaya-error`, its `code` typed by the code vocabulary and pimalaya-cli's common codes, `usage-invalid` and `request-origin`, the item error of an `errors` list under `himalaya-item-error`, its `code` typed by the code vocabulary, and every command printing to stdout under its command key, checked by a test walking the command tree: only a bare `himalaya` (the help), `repl` and `mcp` are left out. A command printing one of two shapes SHALL be described by both, each alternative saying when it prints, and a command printing events SHALL describe them under `<key>.events`. Schemas SHALL describe a value as printed, so a key always printed is required. Every key a schema lists SHALL carry a description, written in JSON terms (`null`, never a Rust `None` nor an intra-doc link), and so SHALL every type its `$defs` describe, a test refusing any description of a schema or of its `$defs` holding an intra-doc link; and a value from a closed set Himalaya defines (`sieve raw` `status`, `pimdir performer` `capability`) SHALL be listed as an `enum`; a set the store or the server may extend (a queue row's `kind`, a mailbox `role`) stays a string. Every generated schema SHALL carry an `$id` naming the contract version, the major version of Himalaya. Every registered payload SHALL be a JSON object, checked by a test. Piped `--json` output SHALL print one value per line.

| Code | Raised when |
|---|---|
| `usage-invalid` | a command line, a REPL line or an MCP tool's arguments the grammar refuses (an unknown flag, command or argument, a missing argument, a bad value); raised by pimalaya-cli beside Himalaya's codes; nothing runs |
| `request-origin` | a `gmail request` or `msgraph request` URL is off the API origin the account's token is for; raised through pimalaya-cli beside Himalaya's codes; nothing is sent |
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
| `store-version` | a pimdir store is of another format version; the message says which is the earlier: sync an earlier one out with its engine and move its directory aside, or upgrade Himalaya for a later one |
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
| `store-account-unknown` | a pimdir store holding collections holds none of the account `pimdir.account` names; nothing is read |
| `row-not-found` | `pimdir queue cancel` names a row the account's queue does not hold; nothing is cancelled |
| `confirmation-required` | a command asks to confirm (`pimdir queue cancel`) with no terminal to answer and no `--yes`; nothing is done |
| `page-token-invalid` | a listing taking `--page-token` (`envelope list|search`, `jmap email|mailbox|submission query`, `pimdir search`, `pimdir attachment list|search`) is given a page token it did not print; nothing is listed |

#### Scenario: A body not downloaded yet
- GIVEN a pimdir item whose body is not local
- WHEN `himalaya --json message read` reads it
- THEN the command exits 1 and prints `{"code":"body-pending","error":"…","sources":[],"backtrace":null}`

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The composers SHALL write the `Date` (the current time at the local offset) and the `Message-ID` themselves, mime-rs's builder writing no field it is not given: the id sits on the domain of the `From` address when it is a plain dot-atom, else on the machine's hostname, else on `localhost`, its left part unique on the machine.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.

With `--json` and neither `--save` nor `--send`, the same composers SHALL print the decoded `from`, `to`, `cc`, `bcc`, `subject`, `date`, `messageId`, `body` and `attachments` instead of the raw bytes, `date` the `Date:` the composed message carries, with its offset, and `messageId` its `Message-ID:` without angle brackets, as the text form prints them in the message's headers, each `null` without one, each address as the envelopes print one, `{name, email}` with `name` decoded or `null`, `from` the first `From:` mailbox or `null`, `to`, `cc` and `bcc` lists of them, groups flattened, and `subject` `null` without one, the body without any signature and with LF line breaks, `attachments` listing each attached file as `{filename, mime, size}` (the template carries no bytes: handing it back means passing the files again through `--attach`), so an editor can lay the message out and hand it back through the flags, sending appending the signature once. `--posting-style none` SHALL send the written body alone, the source left unquoted, for a writer who already laid the quote out.

The text body a composer writes SHALL end with a line break, so a printed single-part message ends on one, and a quoted empty line SHALL be `>` with no trailing blank. `--no-save` SHALL be refused without `--send`. An attached message (`message/rfc822`, from `--attach` or a forward's source) SHALL be written under 7bit, 8bit or binary only (RFC 2046 §5.2.1), its line breaks CRLF, or as the nested message it holds with its own parts encoded when 8-bit content cannot go as is.

### Requirement: A read message is a designed view
`message read --json` SHALL print the row `envelope list` prints for the message, `{id, messageId, inReplyTo, flags, subject, from, to, date, size, hasAttachment}`, then `{etag, headers, text, html, parts}`, `id` the id the command took and `etag` `null`, a message's body never changing and no write taking a version to match: the row read off the raw message as the local stores list it, `size` its bytes and `hasAttachment` whether it attaches a file, its `flags` the ones the backend holds, read in the same exchange (an IMAP `FETCH` asking `FLAGS`, JMAP `keywords`, Gmail labels, the Graph properties, the Maildir file name, the mbox status fields, the pimdir placement), `seen` added by `--seen`; beside it a view built from the raw message so it is the same on every backend, rather than the parser's own model; `message parse --json` SHALL print the same view without the row and `etag`. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset, `Z` for UTC, or `null`, `messageId` without angle brackets, `inReplyTo` and `references` as lists without angle brackets, as envelopes print them, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, a `format=flowed` part unwrapped (RFC 3676 §4.2), joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The message SHALL be parsed by mime-rs, whose structure is the one RFC 2046 delimits: a line opening with a boundary but carrying more than transport padding is no delimiter, an outer delimiter closes the parts nested in it, and a field whose name holds a blank (`Sub ject:`) is not the field it spells. The text output SHALL show the same headers, parts, sizes and names as the JSON view. Its From, To and Cc lines SHALL read the addresses as the JSON lists them, followed by each entry `headers.invalid` lists as `(unreadable: <entry>)`, a header holding neither being left out. `--raw` SHALL write the message bytes verbatim, and print `{data, size, mime}` under `--json`.

### Requirement: Downloaded attachments never replace an existing entry
`attachment download` SHALL check every part id it is given before it writes a file, an id the message does not hold failing the command with nothing written. Attachment downloads SHALL atomically create a new file, readable by its owner alone (0600 on Unix, whatever the umask, so one landing in the shared temporary directory when no `downloads-dir` and no download directory is known stays private), refusing to follow an existing leaf symlink. A file name a message gives SHALL become one path component (pimdir STORAGE §18): `/` and `\`, every control character and every bidirectional formatting character (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069) replaced by `_` by pimalaya-cli's `sanitize::path_component`, the surrounding whitespace and the leading dots dropped first, and a name left empty written `attachment`. A name SHALL be cut to 255 bytes at a character boundary, its suffix and an extension of fewer than 32 bytes kept, so a long name and a full-length name that is taken land rather than fail. An occupied name SHALL be suffixed with a number before its extension. If the original name and suffixes 1 through 1023 are occupied, the command SHALL fail rather than overwrite a file. Concurrent downloads SHALL reserve distinct names. Under `--json` the command SHALL print the attachments written as `attachment list` lists them, each with the `path` it was written to beside its `size`, the bytes written; its text SHALL print one line per file, `Saved <size> bytes to <path>`.

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the receipt of a unified write SHALL carry `queued: true`, a queued send `sent: false`, the message not having left, and its text form SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy`, `flag add|set|remove` and `mailbox create|rename|delete`. A mailbox write SHALL print its `id` `null` while queued, and no queue detail (a row, a queue id) SHALL reach a unified output. Every other backend SHALL print `queued: false`.

#### Scenario: A send on a pimdir account
- GIVEN a pimdir account with `mailbox.alias.sent` set
- WHEN `message send` runs
- THEN it prints `Message queued for sending`, not that the message was sent, and under `--json` `"sent":false,"queued":true`

#### Scenario: A delete on a pimdir account
- GIVEN a pimdir account with a trash mailbox
- WHEN `message delete 3` runs on another mailbox
- THEN it prints `Queued the move of 1 message(s) to the trash`

### Requirement: The raw Maildir folder commands guard the store
`maildir create` SHALL refuse every level of NAME the unified mailbox writes refuse (blank, holding a control character, unlistable), and an existing folder, coded `mailbox-duplicate`; `maildir rename` SHALL refuse the same of its NAME. `maildir rename` SHALL refuse the root, an existing target, and on the fs layout a target whose parent does not exist. `maildir delete` SHALL refuse the root, and removes the folder with every folder below it, nested Maildirs and Maildir++ `.<name>.*` folders included. Each SHALL print `{id}`, the folder's id as the unified `mailbox` commands print and take it, its path: the new one for a creation or a renaming, the former one for a deletion. `maildir list` SHALL print each folder as `{id, name, path}`, `id` that same id and `name` the name relative to the root. `maildir flag set` and `mbox flag set` SHALL require at least one `-f`, refusing the command line (exit 2) rather than clear every flag.

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A unified write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message add`, `message send` and a composer's save or send one send receipt, `{id, saved, sent, queued, errors}`, `id` being the saved or added message's, `null` when the message was not saved or the backend returned none, a queued save naming the link id the store gives it, `saved` whether a copy was saved or added, `sent` whether the message left, `false` without `--send` and while a send is queued. `message delete` SHALL print the receipt of a batch delete, `{action, count, ids, queued, errors}`: it takes several ids, so it counts the messages it acted on as `message copy` and `message move` do, and `ids` names them as given, in that order, the ids of `errors` left out, and `action` says what became of them, `moved-to-trash`, `deleted` from the trash, or `flagged` `\Deleted` with an expunge owing. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page follows the one printed, `null` once the page reaches the end of the listing, an exactly full last page included, and SHALL take it back through `--page-token`, which wins over `--page`. Whether a page follows SHALL come from the listing's own exchanges: one does when the page ends before the count the backend holds (the messages Maildir and mbox list or match, the `EXISTS` of IMAP's `SELECT` for a listing and the UIDs its `SORT` answers for a search, the `total` of JMAP's `Email/query`, the items pimdir counts for a listing), when pimdir's search finds one hit past the page, when the listing of Gmail or Graph holds one message past the page, which the page asks along (`maxResults` or `$top` one longer, a later Gmail page and a Graph search reading the pages before it in as few calls as the largest page allows, the next page token or link followed only while the listing runs short of that message), both answering a next page token or link after an exactly full last page too, and, on a JMAP server computing no `total`, when the page comes back full. Pages SHALL be numbered from 1: `--page 0` is refused with the command line, a page token that is no page number from 1 coded `page-token-invalid`, and `--page-token` given beside `--page` is refused with the command line, exit 2, rather than one of them ignored; `--page-size 0` lists the whole mailbox. `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders` SHALL print Graph's next link (`@odata.nextLink`) as `nextPage`, as Graph gives it, a `$search` page's included, and not beside it, `null` without one or when it leads back to the page asked (`--top 0`); `--page-token` SHALL follow it as is, the query it carries standing for the other options, so a query option given beside it (`--folder`, `--top`, `--skip`, `--filter`, `--search`, `--orderby`, `--select`, `--count`, `--hidden`, the parent folder of `child-folders`) SHALL be refused as a usage error, exit 2, before connecting; a link off the origin of Graph's API (`https://graph.microsoft.com`) being refused before any request, the bearer token never leaving it. `msgraph attachments list` SHALL follow every next link, so it never stops early and prints no cursor. `msgraph messages list --search` SHALL quote its value for Graph, its double quotes escaped, unless it opens on a double quote, and `--count` SHALL keep Graph's `@odata.count` as Graph sends it (`Count:` in text), absent without `--count` or beside `--search`, Graph ignoring `$count` there. The Gmail listings (`gmail messages list`, `threads list`, `drafts list`, `history list`) SHALL print Gmail's `nextPageToken` as `nextPage`, and not beside it, and take it back through `--page-token`; Gmail's other lists hold no cursor. `jmap email query`, `jmap mailbox query` and `jmap submission query` SHALL print `nextPage` beside their collection the same way, a page following while the query's `total` lies past it, or, on a server computing none, while it comes back full, and SHALL take it back through `--page-token`, a token that is no page number from 1 being refused coded `page-token-invalid`, and `--page` beside `--page-token` with the command line, exit 2; `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create`, printing the same shape, SHALL print `nextPage` as `null`.

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

### Requirement: A value a protocol command refuses is refused with the command line
A value of a protocol command's arguments the command would refuse whatever the server answers SHALL be refused with the command line, coded `usage-invalid`, exit 2, before the configuration is read, the secret command run or the session opened: the sequence set of `imap fetch`, `store`, `copy` and `move`; `--mail-from` without `--rcpt-to`, or the reverse, on `jmap submission create`; `msgraph messages update` naming no change; the method of `gmail request` and `msgraph request`; `never` beside another condition in `smtp send --notify`; and the commands `imap raw` is given on the command line, one carrying no tag or a tag another uses, which the exchange could not wait on. An off-origin URL of `gmail request` and `msgraph request` SHALL be refused before the session too, coded `request-origin`, exit 1 (*The Gmail and Graph escape hatches are requests*). A value read from a file or stdin (a raw command, a JMAP request body, a Sieve script) SHALL be read and checked inside the command, a session refusing stdin as `stdin-taken`; `imap raw` SHALL refuse a command read so coded `usage-invalid` too, before anything is sent.

#### Scenario: A sequence set that is none
- GIVEN an IMAP account whose password is a command
- WHEN `himalaya --json imap fetch -m INBOX abc` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, and the password command did not run

#### Scenario: A raw command without a tag
- GIVEN an IMAP account whose password is a command
- WHEN `himalaya --json imap raw -- NOOP` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, and the password command did not run, nothing connecting

### Requirement: The pimdir subcommand reads and retracts the queue
Himalaya SHALL carry a `pimdir` subcommand for what the operator CLI cannot do without knowing mail. `queue list` SHALL list every pending row of the account's queue, whatever its kind (`add`, `submit`, `set-flags`, `move`, `copy`, `remove`, `update`, the collection intents, an application's own), and its parked rows of any kind, mailbox by mailbox in store order, or those of the one mailbox `-m/--mailbox` names (a name, alias, role or id), where the kind-agnostic `pimdir` binary can only print ids and hashes.

Under `--json` it SHALL print `{"actions": [<row>], "parked": [<row>], "errors": [<item error>], "refused": [<refusal>]}`, the pending rows then the parked ones, each in append order and in the family's one queue row (pimalaya-cli cairn/spec/output.md, *A queue row keeps one shape*), every key printed: `{"row", "handle", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject"}`. `row` is the queue row id, an integer, and `handle` the row under the store's identity (`<store id>.<incarnation>.<row>`, read from `PimdirReader::change_cursor`, pimdir STORAGE §4.5), either of which `queue show` and `queue cancel` take, the text printing the handle in place of the bare id; `kind` the row's action as the store holds it, read from io-pimdir (`PimdirAction::kind` for a pending row, `PimdirParkedAction::action` for a parked one), never re-spelled; `collection` the mailbox's id as `mailbox list` prints it; `linkId` the link id of the message an `add` creates, the `id` its receipt printed, `null` for any other row, a `submit` among them, the store creating no message for it; `itemId` the public id of the stored message a `set-flags`, `move`, `copy`, `remove` or `update` acts on, as `envelope list` prints it, `null` for any other row; `producer` the process that staged the row; `queuedAt` the instant the store stamped it; `attempts` how many times the sync engine tried it, an integer from 0 (`uint64`, minimum 0, in its JSON Schema); `unconfirmed` `true` for a row whose outcome is unknown (pimdir STORAGE §15.2); `error` `null` while the row is pending and the store's text once it is parked; `subject` the subject of the body the row pins, else of the message it acts on, a parked `submit` naming no body giving its payload's subject, `null` when there is none to read. The queued action carries no summary, so a subject SHALL be derived from the body the action pins, through the same derivation the owner applies. A parked payload that does not decode addresses nothing: `linkId`, `itemId` and `subject` are `null`. A pending row whose payload does not decode leaves its mailbox's pending rows unread, io-pimdir reading a collection's pending rows whole: the mailbox SHALL be one item error of `errors`, its id the mailbox's, coded `action-malformed`, the other mailboxes still listed, and the command SHALL exit 3. No placeholder SHALL stand in for the id a queued message lacks (no `""`, `0` nor row id).

`refused` SHALL list every refusal standing in what is listed, the account or the one mailbox, (io-pimdir's `PimdirReader::refusals`, pimdir STORAGE §15.4), per message and source, `{"collection", "itemId", "source", "reason", "refusedAt"}`: a change the store applied that a source's server refused for good, which the sync engine holds until the message is changed again or removed. The text form SHALL list each after the parked rows, `refused` in the state column, its instant, the message id, then the source and the reason; `Cancel:` SHALL print only when a pending or parked row is listed, a refusal being no row to cancel. The header SHALL name the mailbox only when `--mailbox` narrows the listing.

#### Scenario: A queued send
- GIVEN a pimdir account holding `imap/INBOX` and `imap/Sent`, and a send queued on `imap/Sent`
- WHEN `himalaya --json pimdir queue list` runs, then `himalaya --json pimdir queue list -m imap/INBOX`
- THEN the first lists the `submit` row of `imap/Sent` under `actions`, and the second lists no row

#### Scenario: Every kind is listed
- GIVEN a pimdir account where `message add`, `message send`, `flag add` and `message move` ran
- WHEN `himalaya --json pimdir queue list` runs
- THEN `actions` holds an `add`, a `submit`, a `set-flags` and a `move` row, the last two naming the message as `itemId`, `parked` and `errors` empty

`queue cancel` SHALL retract one pending or parked row of the account's queue, confirming first unless `--yes`, and SHALL name the row's action kind in its prompt and output: `{"row", "handle", "kind", "unconfirmed"}`, `row` the row id and `handle` the row under the store's identity, a bare id given named under this store's. An unconfirmed row (pimdir STORAGE §15.2) is cancelled as a parked one, which undoes nothing its performer sent: the prompt and the plain output SHALL say it was sent and may have taken effect, and `unconfirmed` is `true`. A row anchored on a collection that is not one of the account's mailboxes (another account's, or an address book or calendar another app of the store queued on), or one neither pending nor parked, SHALL be refused before anything is opened or asked, and stays as it was, coded `row-not-found` unless it was applied. With no terminal to answer and no `--yes`, the cancel SHALL be refused coded `confirmation-required` before anything is opened.

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
- THEN the command fails with the code `store-version`, saying the store is the earlier and to sync it out then move its directory aside, and neither the store's stamp nor the row changes

#### Scenario: Another account's row
- GIVEN a store shared by two accounts and a row queued on the other account's collection
- WHEN `himalaya pimdir queue cancel` runs on that row
- THEN the command fails saying the row is not in the account's queue, and the row stays queued

#### Scenario: A flag change a server refused
- GIVEN a pimdir flag change the sync engine applied, then the IMAP source's server refused for good
- WHEN `himalaya --json pimdir queue show <ROW>` and `pimdir queue list` run on its row and its mailbox
- THEN the row reads `applied` on its message with one entry of `refusals` naming `imap` and the server's reason, and the mailbox lists the message under `refused`

### Requirement: pimdir writes name their queue row
`pimdir message add` and `pimdir message send` SHALL stage a message as the unified `message add` and `message send` do, and print the message's link id as `id`, the id the unified `message add` prints and the row's `linkId`, beside the queue row it waits in: `{"id": string, "row": integer, "handle": string}` for an add, plus `"copy": bool` (the `submit` row carries the copy) and `"copyRow": integer` with `"copyHandle": string` (a copy queued beside the send, when its performer declares no capabilities, both `null` otherwise) for a send. `pimdir flag add|set|remove` SHALL print the row of each message they found and what a source supports of the change only in part: `{"messages": [{"id": string, "row": integer | null, "handle": string | null}], "warnings": [string]}`, both `null` for a message the change left as it was, for which nothing was staged, each warning once, in the words of the source's declaration (`Source gmail supports mail.flags.keywords in part: …`), `Warning:` lines after the sentence in text. The unified commands SHALL keep printing no queue detail. `pimdir queue list`, `queue show` and `queue cancel` SHALL name a row `row` and `handle` as well.

`pimdir queue show <ROW>` SHALL say where a row of the account's queue stands, read through io-pimdir's `action_status` (pimdir STORAGE §15.4), printing the row `pimdir queue list` prints for it, every key of it, then what only a read carries (pimalaya-cli cairn/spec/output.md, *A queue read prints the list row*): `{"row", "handle", "kind", "collection", "linkId", "itemId", "producer", "queuedAt", "attempts", "unconfirmed", "error", "subject", "state": "pending" | "parked" | "unconfirmed" | "applied", "appliedAt", "refusals"}`, a key the state does not carry being `null`, `refusals` `[]`. A pending, parked or unconfirmed row carries the keys of its list row, a parked one `error` too; an applied one, left the queue, carries `collection`, `appliedAt` and `itemId`, the public id of the message it acted on (the one an add, a move or a copy left, the one a flag change or a removal named), the keys of its list row `null`, and `refusals`, read with the receipt (`PimdirActionStatus::Applied::refusals`, pimdir STORAGE §15.4): one `{"collection", "source", "reason", "refusedAt"}` per source whose server refused for good the change the row led to, the plain output printing `Refused by <source> in <collection> at <instant>: <reason>` under the applied line. A refused row reads `applied` all the same: its effect is in the store, so it is neither parked again nor cancelled, and the way out is an action on the message. An intent its performer sent without learning its outcome (pimdir STORAGE §15.2), a send or a mailbox change that may have taken effect, SHALL read `unconfirmed`, never `parked`, with `collection`, `kind`, `attempts` and, as `error`, what the performer recorded; the plain output says it was sent and maybe performed. A handle another store drew (§4.5), a copy restored since, where its id may name another row, SHALL fail coded `store-reset`, exit 1. A row neither queued nor in the store's receipts (kept seven days), or one anchored on a collection that is not one of the account's mailboxes (another account's, or another app's address book or calendar), cancelled or applied long ago, SHALL fail coded `row-not-found`, exit 1. A row the sync engine performed rather than applied, a message sent or a mailbox created, renamed or deleted, reads as `applied` without `itemId`, its receipt recorded on acknowledgement (pimdir STORAGE §15.5). A receipt follows the collection its row was queued on and outlives it (pimdir STORAGE §15.4): a renamed mailbox's row reads `applied` under the mailbox's new id, and a row whose mailbox is gone since, a performed deletion's included, still reads `applied`, with `collection` `null`, the plain output saying the mailbox was deleted since. Such a row names no collection an account could be told from, so it reads `applied` under any account of the store.

The plain output of every `pimdir` subcommand SHALL end with a newline.

#### Scenario: A staged draft is followed by its row
- GIVEN a pimdir account
- WHEN `himalaya --json pimdir message add -m imap/Drafts` stages a message
- THEN it prints `{"id":"d1@x.org","row":12,"handle":"e2646df031cd876334410c4df14ebd86.1.12"}`, and `pimdir queue show e2646df031cd876334410c4df14ebd86.1.12` prints its list row and `"state":"pending"` until the sync engine applies the row, then `"state":"applied"` with the `itemId` the message is read by

#### Scenario: An unknown row
- GIVEN a pimdir account whose queue holds no row 999
- WHEN `himalaya --json pimdir queue show 999` runs
- THEN it prints the failure object coded `row-not-found` and exits 1

### Requirement: The pimdir queue view shows parked rows
`himalaya pimdir queue list` SHALL list the parked rows of what it lists, any action kind, beside the pending rows, in the one queue row ("The pimdir subcommand reads and retracts the queue"), under `parked` in JSON: the row id, the action kind, the message it acts on, when it was queued, whether it is unconfirmed (`unconfirmed`, pimdir STORAGE §15.2) and the error that parked it. The plain output SHALL list an unconfirmed row as `unconfirmed` where it lists another as `parked`. `queue cancel` SHALL take a parked row as it takes a pending one.

#### Scenario: A duplicate draft parks
- GIVEN a staged `add` the sync engine parked because the source listed the same `Message-ID` before applying it
- WHEN `himalaya pimdir queue list -m imap/Drafts` runs
- THEN the row is listed as parked with its error, and `pimdir queue cancel` on its id removes it

### Requirement: pimdir creates a mailbox by queueing an intent
`pimdir mailbox create <NAME> [--parent <MAILBOX>]` SHALL enqueue one `collection-create` intent (pimdir STORAGE Annex B.2) with the `v: 1` payload `{"v": 1, "source": id, "name": text, "parent": collection?}`, and print `{"id": null, "row": integer, "handle": string, "name": text, "parent": collection | null, "source": id}`, `id` `null` until the sync performing the row brings the mailbox down, `parent` `null` at the top, never left out. `pimdir mailbox rename` and `pimdir mailbox delete` SHALL print `id` too, the id of the mailbox they rename or delete, beside their row. It SHALL NOT create a collection in the store: the mailbox arrives with the sync that performs the intent.

The row SHALL be anchored on `parent`, which must be a mailbox of the account, else on the account's first mailbox by id, and queued through io-pimdir's `enqueue_collection_create`. The performer SHALL be named as for a send (capability `collection.create`): several candidates and no recorded choice are refused, naming them. A store whose sources declare no capabilities SHALL be refused coded `capability-missing`, its owner predating the intent, as SHALL a renaming or a deletion on such a store. An empty name, or one holding a control character, SHALL be refused before anything is staged. A mailbox the store lists already SHALL be refused with `mailbox-duplicate`, naming it, before anything is queued, as "Unified mailbox writes" refuses creating an existing mailbox: one the performer's source syncs under NAME at the top, or under the parent's name, a hierarchy delimiter (`/` or `.`, the store recording none) and NAME, names compared exactly; so SHALL a creation of NAME under that parent the same source has queued already. A name the store does not list yet is the server's to refuse: the row parks, and `pimdir queue show` says why.

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

### Requirement: Unknown keys are refused
Every table of the configuration SHALL refuse a key it does not know, the top level and the account blocks included, the error naming the key and the line. The one exception SHALL be the `tui` table, at the top level and in an account, which holds himalaya-tui's own keys: Himalaya SHALL load it whatever it holds and read none of it. A key or a table an earlier version read SHALL be refused as any unknown key is, through serde's `deny_unknown_fields` alone, never by name: the v2 `table`, `attachment`, top-level `account` and `envelope.list.table`, v1's `folder` and `backend`, and the `m2dir` block among them; MIGRATION.md says what replaced each.

#### Scenario: A typo at the top level
- GIVEN a configuration setting `downloads-dri` at the top level
- WHEN any command reads it
- THEN it is refused naming `downloads-dri` and its line

#### Scenario: A v1 folder alias
- GIVEN an account setting `folder.alias.inbox = "INBOX"`
- WHEN any command reads it
- THEN it is refused naming the unknown field `folder` and its line

#### Scenario: An m2dir block
- GIVEN an account setting `maildir.root` and `m2dir.root = "~/Mail"`
- WHEN any command reads it
- THEN it is refused naming the unknown field `m2dir`, rather than loading as if the block were absent

#### Scenario: himalaya-tui keys
- GIVEN a configuration holding `[tui]` with `keybinds`, `contact-command`, `contact-complete-key` and `theme`, and `[accounts.work.tui]` with `contact-command`
- WHEN any command reads it
- THEN it loads, and the same keys outside a `tui` table are refused

### Requirement: No colour is configured
The configuration SHALL hold no colour, no table shape and no glyph. The v2 blocks holding them SHALL be refused as unknown keys, as "Unknown keys are refused" says: `table` (`table.preset`, `table.arrangement`) and `attachment` at the top level and the account level, `account` at the top level, and `envelope.list.table`, whatever they hold; `mailbox.list` is an unknown key of `mailbox`. `envelope.list` SHALL keep refusing an unknown key of its own.

#### Scenario: A v2 glyph key
- GIVEN an account setting `envelope.list.table.unseen-char = "N"`
- WHEN any command reads the configuration
- THEN it is refused naming the unknown field `table` and its line

#### Scenario: A typo under envelope.list
- GIVEN an account setting `envelope.list.datetime-fnt`
- WHEN any command reads the configuration
- THEN it is refused naming `datetime-fnt`

### Requirement: Writing and sending are opt-in
Without flags the server SHALL list reading tools only, without the arguments that write. `--allow-write` SHALL add the tools and arguments that change the store, `mailbox_create`, `mailbox_rename` and `mailbox_delete` among them. `--allow-send` SHALL add those that send mail, `vacation_set` among them, an auto-reply once on answering mail, and the arguments that say how a message is sent, `send` and `no-save`. Under `--allow-send` without `--allow-write`, a tool sending (`message_send`, the composers) SHALL run with `--no-save` fixed, its description saying so and `no-save` being no property, since filing the copy `message.send.save-copy` configures is a write; a transport filing every message it sends itself (Graph), whose copy is the provider's and which refuses `--no-save`, runs without it.

#### Scenario: A send allowed, writes not
- GIVEN a Maildir account sending over SMTP whose `message.send.save-copy` names `Sent`
- WHEN `himalaya mcp --allow-send` serves a `message_send` call
- THEN the message is sent, the receipt says `"saved":false`, and `Sent` holds no copy; under `--allow-send --allow-write` the copy is filed Under `--allow-send` a sent message MAY carry `attach-data`, so the flag lets an agent send arbitrary binary data and not only text.

## REMOVED Requirements

### Requirement: A v1 account layout is named, not silently ignored
Replaced by *A missing backend is named*: the v1 `backend` table is an unknown key.

### Requirement: An m2dir block is named, not loaded
Replaced by *No m2dir store is read*: the `m2dir` block is an unknown key.
