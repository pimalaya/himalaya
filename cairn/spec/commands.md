---
cairn: spec
capability: commands
status: current
---

# Commands

The command tree splits into three groups. The unified API (mailbox, envelope, flag, message, attachment, vacation) is the cross-protocol surface: the union of what the backends serve, held coherent by one meaning per verb, a backend that cannot give a verb's result declaring it unserved rather than giving another. The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir, mbox, pimdir, smtp, sieve) each expose the full surface of one backend, including operations the unified API cannot model. The meta commands (account, repl, mcp, completion, manual, json-schema) cover account configuration, a session running many commands (cairn/spec/repl.md), an MCP server (cairn/spec/mcp.md), shell completions, man pages and JSON Schemas.

### Requirement: Unified commands over a resolved transport
The unified commands SHALL run over an `EmailClient` owning one backend-client variant per compiled-in backend, each verb resolved to a configured transport. The verbs exchanging ids resolve to the storage backend: the first configured one the global `--backend` flag allows, local before network (Maildir, mbox, pimdir, JMAP, Gmail, Graph, IMAP). Sending resolves to the storage backend when it sends (JMAP, Gmail, Graph, pimdir), else to the SMTP transport, which SHALL connect for each send and close after it. `--backend` SHALL filter the resolution. Each shared method matches the resolved backend and calls its per-protocol adapter.

### Requirement: Capability registry
Each backend SHALL declare, in one table per backend (src/capability.rs), the unified capabilities it serves, named in pimdir STORAGE Annex B's grammar without the domain word (`message.move`, `flags.answered`, `submit`, and `envelope.search.<option>` for each clause and sort key of the query), at a level: `full`; `partial`, with a detail saying what differs; `server`, with a detail saying what the server must advertise; or `none`, with a detail saying why and what serves it instead. A backend SHALL declare a capability served only when its result is the one every other declarer gives, and declare `none` otherwise. The tables SHALL be compiled into every build, compiled-out backends included.

### Requirement: An unserved verb is refused before connecting
A unified command SHALL be checked against the capabilities its arguments reach (each flag it names, each clause and sort key of its query, `--save`, `--send`, `--seen`) before any connection, on the resolution of the account's blocks or of the kept session client, so a refusal resolves no credential, opens no TLS and drops no kept client. A capability no configured transport answers, or one its transport declares `none`, SHALL fail with the code `capability-missing` and a message naming the account, the capability, the backend or the `--backend` filter, and the detail; a query option SHALL fail with `search-unsupported` instead. A `--backend` naming a block the account does not configure SHALL fail with `capability-missing` too. A copy `message.send.save-copy` configures SHALL be checked once the account is read, still before the send. An account configuring no transport at all SHALL keep the error naming its blocks. A block of a backend the build leaves out SHALL load, so one file serves every build, and a command that would use it, the account holding no other block that serves the command or `--backend` naming it, SHALL fail coded `backend-not-built`, naming the cargo feature that builds it.

#### Scenario: Answered on Gmail
- WHEN `himalaya flag add 42 --flag answered` runs on a Gmail account
- THEN it fails with `capability-missing` naming `flags.answered`, before the token command runs, where it used to report the message flagged and change nothing

### Requirement: A server-decided capability is refused once connected
A capability declared `server` SHALL be checked by the adapter once connected, before any write. A JMAP send whose session lacks `urn:ietf:params:jmap:submission` (RFC 8621 section 7) SHALL fail with `capability-missing` before the upload, leaving no draft. An IMAP removal without UIDPLUS SHALL flag the messages `\Deleted` and say so. A JMAP `vacation` whose session lacks `urn:ietf:params:jmap:vacationresponse` SHALL fail with `capability-missing`, and an IMAP keyword the mailbox would not keep SHALL be refused as "An IMAP keyword is kept or refused" says.

### Requirement: Verbs exchanging ids share one transport
The unified verbs that take or return message, envelope or mailbox ids (mailbox, envelope, message, flag, attachment) SHALL resolve to the same storage transport for a given account and `--backend`, so an id one of them returns is valid in the others. A verb exchanging no such id, sending or the auto-reply, SHALL resolve on its own.

### Requirement: An account feature resolves on its own
A capability of the account rather than of a message, the auto-reply, SHALL resolve to the first backend of the storage order the account configures and `--backend` allows that serves it, else to the storage backend, whose refusal names what serves it instead.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `vacation `msgraph mail-folders list|child-folders`: the id, the counts, the child folder count, the name, paged by token; `msgraph mail-folders get` as fields, `Id:`, `Name:`, `Parent:`, `Messages:`, `Child folders:`.` runs
- THEN it reads JMAP's auto-reply, where `envelope list` reads the Maildir

### Requirement: The unified command tree is the same in every build
The unified command tree SHALL be compiled in every build carrying a mail transport (any storage backend, or SMTP alone), whichever backends it compiles, dispatch alone being gated; a verb no compiled backend serves parses and fails with `capability-missing`. The unified commands' `json-schema` output and the error code vocabulary SHALL be identical across such builds. Each unified leaf's `--help` SHALL end with a `Backends:` footer generated from the tables, listing per capability it reaches the backends at each level and the details, then the capabilities only an argument reaches, each once, naming the arguments (`submit (with --send, --no-save)` on the composers). A protocol namespace exists with its feature.

### Requirement: Resolution changes no served verb
Resolving through the registry SHALL leave the output of every verb a backend serves unchanged. Only a verb a backend declares `none` changes: from a silent no-op (`flag add answered` and `draft` on Gmail and Graph), an uncoded error (`message add` on Graph) or a failure after the send (`message send --save` on Graph) to `capability-missing` up front.

### Requirement: Protocol commands ignore backend selection
Each protocol command SHALL build its own `<Proto>Client` via a `build_<proto>_client` helper and run against that backend directly, ignoring `--backend`. The imap command mirrors IMAP's flat command list; gmail and msgraph track their REST resource domains; the filesystem backends expose only operations that map to their on-disk layout, leaving MIME rendering to the unified commands.

### Requirement: ManageSieve commands

When built with the `sieve` feature, the protocol-specific API SHALL expose `sieve capability`, `list`, `get`, `put`, `check`, `activate`, `deactivate`, `rename`, `delete`, and `raw`. Every command SHALL print a dedicated serializable output type: `activate`, `rename` and `delete` the receipt `{id}` naming the script, `activate` refusing an empty name as a usage error, exit 2, before connecting (RFC 5804 reads `SETACTIVE ""` as a deactivation), `deactivate` `{active: null}`, `check` and `put` `{name, warnings}`, `warnings` being the lines of the text a server attaches to an accepted script, one entry per line, `[]` when it sends none, and `name` `null` for a check. `put` and `check` SHALL accept script bytes from a file, inline arguments, or stdin.

The protocol SHALL live in io-managesieve, so this repository holds a `SieveClient` wrapping `ManagesieveClientStd`, one file per subcommand, and its own serializable output types, exactly as the other protocol modules do.

### Requirement: Raw passthrough is byte-verbatim
The `imap raw` and `smtp raw` commands SHALL forward the command bytes to the server verbatim, resolving the argument through the shared `RawCommandArg` (positional or stdin read whole, refused when not valid UTF-8). It decodes literal `\r` / `\n` escapes into real CRLF so a shell-typed command survives intact. `imap raw` sends a batch of caller-tagged commands: it appends a trailing CRLF when missing and delegates tagging, framing and out-of-order completion tracking to io-imap. `smtp raw` stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects a multi-line batch, since the SMTP exchange reads exactly one reply. `sieve raw` sends one command line and reads the complete ManageSieve response; literal-bearing Sieve commands use their typed commands so the response reader remains synchronized.

The reply SHALL be printed split into what himalaya understands of it. Under `--json`, `imap raw` SHALL print each untagged response as an `untagged` event `{kind, number, text}` (`* 3 EXISTS` being kind `EXISTS`, number 3), each continuation request as a `continuation` event `{text}`, and a response it cannot split as an `unparsed` event `{text}`, a literal kept in the response announcing it; then the result `{completions: [{tag, status, code, text}]}`, and `sieve raw` each data line as a `data` event `{text}`, then the completion `{status, code, text}`. The text form of `imap raw` and `sieve raw` SHALL print every response line as received, untagged and data lines included, as the result, the events being printed under `--json` only. `smtp raw` SHALL print `{code, text}`, the reply's lines joined, `code` `null` when the reply opens with none. A refused completion (an IMAP `NO` or `BAD`, a ManageSieve `NO` or `BYE`, an SMTP 4xx or 5xx) SHALL exit 3, its result printed. `imap raw` reads the whole reply before printing, so its events come before the result but not as they arrive, and io-imap decodes the reply lossily, so a non-UTF-8 byte comes back as U+FFFD. A literal holding one has no knowable octet count, so `imap raw` SHALL frame the reply up to the response announcing it and print that response and everything after it as one `unparsed` event (one block in text), never splitting a line of the literal into a response, then the completions read before it, and exit 3.

### Requirement: The JMAP escape hatch is a request
When built with the `jmap` feature, `jmap request <JSON>` SHALL send one JMAP request (RFC 8620 section 3.3) given as a whole request object, `{"using": [...], "methodCalls": [...]}`: a path to a JSON file, the JSON itself, or `-` for stdin (refused inside a session as `stdin-taken`), `using` the core and mail capabilities when absent, the calls sent as written. It SHALL print `{response}`, the server's `{methodResponses, createdIds, sessionState}` as it answered them, its text form that object pretty-printed, as Cardamum's `jmap request` does. `request` is the escape hatch of an HTTP API across the family, `raw` the one of a line protocol (pimalaya-cli cairn/spec/commands.md). `jmap query [--using <URN>]... [METHOD_CALLS]...`, its former form, SHALL stay, hidden from the help and the manual, under its schema key `himalaya-jmap-query`: it sends one request whose `methodCalls` is the JSON array given, read from stdin when it is `-` or absent (refused inside a session as `stdin-taken`), whose `using` is the core and mail capabilities plus each `--using`, and whose calls naming no `accountId` get the session's primary mail account, and prints `{methodResponses}`, each response `[name, arguments, callId]` as the server answered it, its text form the same array.

#### Scenario: The former form
- GIVEN a JMAP account
- WHEN `himalaya --json jmap query '[["Mailbox/get", {}, "c0"]]'` runs, then `himalaya --json jmap request '{"methodCalls": [["Mailbox/get", {"accountId": "a1"}, "c0"]]}'`
- THEN the first prints `{"methodResponses": [...]}` and the second `{"response": {"methodResponses": [...], "createdIds": null, "sessionState": "..."}}`, and `himalaya jmap --help` lists `request` alone

### Requirement: The Gmail and Graph escape hatches are requests
When built with the `gmail` or `msgraph` feature, `gmail request <METHOD> <PATH> [JSON]` and `msgraph request <METHOD> <PATH> [JSON]` SHALL send one HTTP request under the account's bearer token, as Cardamum's `gpeople request` and `msgraph request` do: METHOD `get`, `post`, `patch` or `delete`, `put` too on Gmail, in any case, another refused with the command line, exit 2; PATH joined onto `https://gmail.googleapis.com/gmail/v1/` or `https://graph.microsoft.com/v1.0/`, a leading `/` dropped, or a full URL taken as it is when it stays on that API's origin (`https://gmail.googleapis.com`, `https://graph.microsoft.com`), a URL off it refused with the command line, exit 2, the bearer token never leaving for another host; JSON the body of a `post`, `put` or `patch`, `null` when absent. They SHALL print `{response}`, the body the provider answered as it sent it, `null` for an empty one, its text form the body pretty-printed, nothing for an empty one. A refusal SHALL print as the typed commands' refusals do, under `refusal`, exit 3 (*A protocol refusal is a result*).

#### Scenario: A Graph patch
- GIVEN a Microsoft Graph account
- WHEN `himalaya --json msgraph request patch /me/mailFolders/F1 '{"displayName":"Work"}'` runs
- THEN it sends `PATCH /v1.0/me/mailFolders/F1` with that body and prints `{"response": <the folder Graph answered>}`

### Requirement: Account threaded as a sibling argument
The active account context SHALL be threaded as a sibling argument through every `execute` chain, never reached through the client. Subcommands receive `account` and `client` side by side.

### Requirement: account check covers every declared backend
`account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole, pimalaya-cli's `account::AccountCheckOutput`, one row `{backend, ok, code, error}` per backend, `code` and `error` `null` when the backend answered, a failure coded as a one-shot run codes it. When any fails, it SHALL exit 3 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/. A Maildir block SHALL be checked by listing its store, a missing root failing the check.

### Requirement: An account list names every block
`account list` SHALL print pimalaya-cli's `account::AccountListOutput`, one row per account, `{name, default, backends, notBuilt}`, sorted by name: `backends` naming each block the account declares that the build serves, `notBuilt` each block it declares that the build leaves out, both among `imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`, `smtp` and `sieve`, in that order. Its text SHALL be pimalaya-cli's: the name, the backends joined with `,`, each one left out followed by ` (not built)`, and `default` on the default account. It SHALL load the configuration without taking an account.

#### Scenario: A pimdir store
- GIVEN an account declaring a `pimdir` block and an `smtp` block
- WHEN `account list` runs
- THEN its BACKENDS cell reads `pimdir,smtp`

#### Scenario: A backend the build leaves out
- GIVEN a build without `mbox` and an account declaring a `maildir` block and an `mbox` block
- WHEN `account list` runs
- THEN its row prints `"backends":["maildir"],"notBuilt":["mbox"]` under `--json`, and its BACKENDS cell reads `maildir,mbox (not built)`

### Requirement: Output discipline
Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs, the welcome a person meets without a configuration (cairn/spec/config.md), the REPL's prompt and its help in a JSON session, and the failures of `mcp`, whose stdout carries protocol messages only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.

### Requirement: Text output is plain text
A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

Every list SHALL print through pimalaya-cli's `Columns` with no header row, one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-` and a line break inside a cell as a space. Its short cells SHALL read first and its free text (a subject, a name, a path, an error) last; a cell holding several values SHALL join them with `,`, a size SHALL print with no blank (`1KiB`), a mailbox's unread and total counts SHALL print as one cell, unread first (`3/120`), a count the backend gave none of as `-`, and a yes-or-no cell SHALL print its word (`default`, `active`, `permanent`, `inline`, `subscribed`) or nothing, so a line splits on blanks.

Above the items SHALL print `Account:` (the account name, then in parentheses the backend answering the whole list) and the fields naming what is listed (`Mailbox:`, `Message:`, `Query:` and the others below). After them, a paged list SHALL print `Page:`: a list paged by number names the page and the items it holds (`2, envelopes 26 to 50`, followed by `, the last` when no page follows, or `empty`; `all` for an unpaged listing); a list paged by token counts them (`25 messages`, `, the last`, `empty`). The other fields placing the answer (`Incomplete:`, `History id:`, `Cancel:`) SHALL follow in the same block, and a blank line SHALL part the block from the items. While a page follows, `Next page:` SHALL print after the items, one blank line below them and above the errors of the items the command could not serve ("An item that failed prints last"), with the flag reaching it, `--page` for a list paged by number and `--page-token` for one paged by token (pimalaya-cli 0.6.0, ruling D27 of the family's alignment); nothing else SHALL print under the items. Every list SHALL print through pimalaya-cli's `listing::Listing`, its field values starting in one column, two spaces past the colon of the longest label; `Incomplete:` and `History id:` widen it. A list reading no account (`account list`, `maildir flag list`, `mbox flag list`) SHALL print its items alone; a list without items, its header alone, unless it says otherwise below.

A list of messages SHALL print each the way `mscan` prints a mailbox: its id; four flag columns, `*` for no `\Seen`, `R` for answered, `!` for flagged and `@` only when the backend says the message has an attachment, each a space otherwise; its date as `envelope.list.datetime-fmt` (`%F %R` by default) and `envelope.list.datetime-local-tz` (true by default) configure it; the list's own short cells; its senders' display names, the address standing for a nameless one, joined with `, `; then its subject, last. These lists print so:

- `envelope list|search`: the envelopes in the listing's order; the senders, or `to ` and the recipients with `--recipient`. The size SHALL NOT print. Above: `Mailbox:` (its alias resolved) and, for a search given a query, `Query:` (the query as parsed). Paged by number.
- `jmap email query|get`: the date is `receivedAt`. Above: `Mailbox:` when `--mailbox` filters. A query is paged by number.
- `msgraph messages list`: the date is `sentDateTime`, else `receivedDateTime`. Above, `Folder:`, `Search:` and `Filter:` when given, and `Count:` with `--count`. Paged by token, `Next page:` giving Graph's next link.
- `pimdir search`: its mailboxes, joined with `,`, then, when a text term matched any hit, `body` or `headers`; the snippet SHALL NOT print. Above: `Query:`. Paged by number; after the page, `Incomplete:` and what the answer could not see, when it is not whole.
- `imap thread`: each thread from its first message, the subject indented two spaces per level: a run of members nests each under the one before, the answers sit under the last, and the answers of a message the mailbox lacks stand side by side at its level. A message whose envelope could not be fetched prints its id, its other cells empty. Above: `Mailbox:` unless `--no-select`.
- `pimdir thread`: each message after the earlier one its `In-Reply-To` names, its subject indented one level further, one whose parent the account lacks standing at the margin; its mailboxes, joined with `,`, before the senders. Above: `Thread:` when a thread holds the message.

`pimdir tag` and `pimdir queue list` SHALL print the same shape with what they carry. `pimdir tag`: the message id; the flags once the tag applies, blank where they were never read; the status (`planned`, `queued`, `unchanged`, `flags-unknown`, `several-sources`, `not-synced`, `unsupported`); the mailbox; the queue row; the subject. Above: `Query:` and `Incomplete:` as for a search. `pimdir queue list`: the row; `queued`, `parked` or `unconfirmed`; when it was queued; the row's `kind`; then the subject of a queued row, else the id of the message it acts on, or why the row was parked. Above: `Mailbox:` and `Cancel:` with the command cancelling a row, when any is listed.

The other lists SHALL print these cells, in this order:

- `mailbox list`: the id, the role, with `--counts` the counts, the name.
- `attachment list|download`: the part id, `inline`, the size, the type, then the filename, or the path once written. Above: `Mailbox:` and `Message:`. Without any part to list, `No attachments` in place of the items.
- `account list`: the name, the backends, `default`. `account capability`: the capability, the backend, the level, the detail; `Account:` names no backend.
- `sieve list`: `active`, the name.
- `imap list`: the delimiter, the role, the attributes, the name. `imap flag`: the flag, `permanent`; above, `Mailbox:`. `imap search|sort`: one id per line; above, `Mailbox:` unless `--no-select`.
- `maildir list`, `mbox list`: the name, the path. `maildir flag list`: the letter, the name, in the letters' ASCII order (`D`, `F`, `P`, `R`, `S`, `T`), as `-f` lists its values; `mbox flag list`: the letter, the header, the name.
- `jmap mailbox query|get`: the id, the role, the counts, `subscribed`, the name; a query paged by number. `jmap submission query|get|create`: the id, the email id, the identity id, the undo status, the send date; a query paged by number. `jmap identity get`: the id, the email, the name. `jmap thread get`: one email per line, beside its thread.
- `gmail messages list`: the id, the thread id. `gmail threads list`: the id, the history id, the snippet. Both above: `Labels:` and `Query:` when given. `gmail drafts list`: the draft id, the message id, the thread id; above, `Query:`. The three paged by token. `gmail labels list`: the id, the type, the counts, the name; `gmail labels get` the same as fields, `Id:`, `Type:`, `Messages:`, `Name:`.|get`: the id, the type, the counts, the name. `gmail history list`: one line per message change, the record id, `added`, `deleted`, `labeled` or `unlabeled`, the message id, then the labels; above, `Since:` and `Label:`; paged by token, counting records; after the page, `History id:` and the mailbox's new history id. `gmail settings filters list`: the id, the criteria, the action. `gmail settings sendas list`: `default`, the verification status, the address, the name. `gmail settings delegates list` and `forwarding-addresses list`: the verification status, the address. A Gmail snippet SHALL print in text with its `<b>` marks stripped and its HTML entities decoded.
- `msgraph attachments list`: the id, `inline`, the size, the type, the name; above, `Message:`. `msgraph mail-folders list|get|child-folders`: the id, the counts, the child folder count, the name; `list` and `child-folders` paged by token.
- `pimdir mailbox list`: the id, the role, the counts, the coverage, the round under way, the name. `pimdir flag list`: one flag per line; above, `Mailbox:` and `Message:`. `pimdir attachment list|search`: the file id, the size, the type, the part, the message id, the mailbox, the date, the sender's name or address, the filename; above, `Query:` for a search; paged by token. `pimdir search --threads`: the thread id, its message count, its last date, the ids of its hits joined with `,`, the senders and subject of its first hit; above, `Query:`; paged by number.

A record SHALL print through pimalaya-cli's `Fields`: one `Label: value` line per field, the values aligned past the longest label, a value of several lines continuing under its first. Several records SHALL print a blank line apart. The template a composer prints without `--save` or `--send` SHALL stay the RFC 5322 template an editor round-trips.

#### Scenario: An envelope list
- GIVEN the IMAP account `work` whose inbox holds an unseen flagged message `42` from Alice with an attachment, then a seen answered one `41` from Carol, on pages of 25
- WHEN `envelope list` prints them
- THEN it prints `Account:    work (imap)`, `Mailbox:    INBOX`, `Page:       1, envelopes 1 to 2, the last` and a blank line, then `42  * !@  ` followed by the date, `Alice` and the first subject, then `41   R    ` followed by the date, `Carol` and the second subject, aligned, with no border and no colour

#### Scenario: A thread
- GIVEN an IMAP `THREAD` answering `(1 (2 4)(3))`
- WHEN `imap thread` prints it
- THEN the lines read 1, 2, 4 and 3, the subject of 1 at the margin, those of 2 and 3 indented two spaces and that of 4 four

#### Scenario: A mailbox list
- GIVEN the IMAP account `work` holding two mailboxes
- WHEN `mailbox list` prints them
- THEN it prints `Account:  work (imap)` and a blank line, then one aligned line per mailbox ending with its name, with no header row and nothing under it

#### Scenario: A page reached by a token
- GIVEN a Gmail account `work` whose drafts fill more than one page
- WHEN `gmail drafts list` prints the first
- THEN it prints `Account:  work (gmail)`, `Page:     ` with the number of drafts it holds, a blank line, one line per draft, then a blank line and `Next page: --page-token ` followed by the token

### Requirement: Help is plain text
`--help` and `-h` SHALL print without colour, each paragraph on one line, never wrapped to the terminal width.

### Requirement: A query error is drawn without colour
The report `envelope search` and the pimdir search commands draw over a query they refuse SHALL carry no colour escape, on a terminal as in the `--json` error string.

### Requirement: A JSON error carries a stable code
A failure a caller is expected to act on SHALL carry a stable code, printed under `--json` as a `code` string beside `error`, `sources` and `backtrace`. The code SHALL be found anywhere in the error chain, so added context does not hide it. A failure with no code SHALL print `code: null`. The plain output SHALL stay unchanged. Codes are kebab-case and never renamed; `error` stays free wording.

### Requirement: A server's refusal codes the whole command
A command failing whole because a server refused it SHALL carry the code of that refusal, read from the typed error of the protocol that answered it anywhere in the error chain: an IMAP completion naming `NOPERM` `permission-denied` and one naming `OVERQUOTA` `quota-exceeded` (RFC 5530 §3); a Gmail or Microsoft Graph answer as the same answer codes an item; a JMAP HTTP 429 `rate-limited` and 403 `permission-denied`, whichever request it answered, a method error `forbidden` `permission-denied`, and the SetError refusing the one write of a command by its type (`forbidden`, `overQuota`, `rateLimit`). The code SHALL be given at every exit: a one-shot run, a REPL line, an MCP call, and the row of an `account check`. Inside a protocol namespace the refusal SHALL be the command's result rather than a failure (*A protocol refusal is a result*), its `refusal.code` carrying that code. A command reading one message (`message read`, `attachment list`, `attachment download`, the source of `message reply` and `message forward`) whose id the backend does not hold SHALL fail coded `message-not-found` on every backend, a server's answer kept in `sources`.

#### Scenario: A mailbox the server forbids
- GIVEN an IMAP server answering `CREATE` with `NO [NOPERM]`
- WHEN `himalaya --json mailbox create Denied` runs, or the line `mailbox create Denied` in a `repl`, or the MCP call `mailbox_create`
- THEN each answers the failure object coded `permission-denied`

#### Scenario: A message the mailbox does not hold
- GIVEN a Maildir account
- WHEN `himalaya --json message read 999` runs
- THEN it exits 1 and prints the failure object coded `message-not-found`, naming `999` and the mailbox

### Requirement: The exit status tells the outcome
A command SHALL exit 0 when it succeeds and 1 when it prints the failure object. A command line the grammar refuses (an unknown flag or command, a missing argument, a bad value) SHALL exit 2, parsed through pimalaya-cli's `clap::parse`: with a `--json` word before any `--`, it SHALL print on stdout the failure object coded `usage-invalid`, its `error` clap's message without its `error: ` prefix or styles, and clap's rendering on stderr otherwise. A command that printed its result without fully succeeding (a refused protocol completion, an `account check` with a failed backend, a result whose `errors` is not empty) SHALL exit 3, printing nothing after its result. Help and version SHALL print and exit 0 as clap does.

#### Scenario: A mistyped flag under --json
- WHEN `himalaya --json envelope list --nope` runs
- THEN stdout holds the failure object `{"code":"usage-invalid","error":"unexpected argument '--nope' found…","sources":[],"backtrace":null}`, stderr is empty, and the exit status is 2

#### Scenario: A failed check
- GIVEN an account whose Maildir root does not exist
- WHEN `himalaya --json account check` runs
- THEN it prints the report, the `maildir` row `ok: false`, and the exit status is 3

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

### Requirement: Plain output is printable
A single-line field taken from a message or a server (a subject, a name, a filename, a mailbox name, an id) SHALL have its control characters (C0, DEL and C1) and bidi controls (U+202A to U+202E, U+2066 to U+2069) replaced with U+FFFD before it is printed as plain output, so it can neither drive the terminal nor reorder what is displayed. The designed `message read` and `message parse` views SHALL replace those characters in decoded bodies too, retaining tabs and line feeds and normalizing CRLF to LF, on terminals and redirected stdout alike. JSON strings and explicitly requested raw bytes SHALL remain unchanged. Other byte writers SHALL retain their binary-content guard on terminals.

### Requirement: Generation commands print to the standard output
The `completion`, `manual` and `json-schema` commands SHALL share one shape: a positional list selecting what to generate, defaulting to everything, and an optional `--dir` deciding where it lands. Without a directory they SHALL print the single selected item to the standard output, so a packaging helper can capture it and a shell can redirect it to a file, and SHALL fail when several items are selected rather than concatenating pieces valid for nothing. With a directory they SHALL write one file per selected item in it, creating the directory when missing, and report where each landed.

### Requirement: Data commands serialize their data
Every command SHALL hand the printer a dedicated output type implementing both `Display` and `Serialize`, and register its JSON Schema under the command's invocation key; a sentence is the text form of that data, never a JSON field. Where a sibling `list` already serializes a backend resource, the `get` output SHALL emit that resource verbatim through a transparent newtype, so one item read with `get` has the shape of one row of `list`. A protocol read of one resource SHALL print it as the provider sent it ("Protocol listings print the provider's payload"), its text form listing what it shows from it: `gmail messages get` the message Gmail answered at the format asked, `payload` included under metadata and full, `gmail threads get` the thread with each message as Gmail answered it, its text form listing each leaf part of a full payload with its MIME type, file name, size and attachment id. A Gmail settings write SHALL print Gmail's answer to its `PUT` as Gmail sent it, the shape of the matching `get`. A Gmail list SHALL print the provider's collection under its own key (`delegates`, `filter`, `forwardingAddresses`), and `gmail messages list` its `messages`.

### Requirement: Serialized collections are always present
An output field holding a collection SHALL be serialized even when empty, because the schema marks it required regardless and a skipped field would contradict the published schema.

### Requirement: JSON keys are camelCase
Every output type handed to the printer SHALL serialize its keys as camelCase, matching the wire formats the backends speak (JMAP per RFC 8620, Microsoft Graph, the Google APIs) and keeping every key reachable by dot access in jq and JavaScript. Each one SHALL be named `<Domain><Target><Verb>Output`, the domain omitted for the unified commands. A field carrying a provider spelling verbatim SHALL keep its explicit `#[serde(rename)]`, the provider's names not being derivable ones (a provider's paging cursor, `@odata.nextLink` or `nextPageToken`, being the one key lifted, into `nextPage`: "Listings resume from their cursor"), and a transparent newtype over an io-gmail or io-msgraph resource SHALL serialize that resource as its crate declares it. The rule covers keys only: string values keep their own spelling (error codes, flags, mailbox roles, `message delete` actions and pimdir queue states stay kebab-case or lowercase). Configuration types SHALL stay kebab-case: they are read from TOML and are not `--json` payloads.

### Requirement: An instant has one spelling
Every instant Himalaya writes as RFC 3339 under `--json` (an envelope's `date`, the `date` of `message read` and `message parse`, the `internalDate` of `imap fetch`, the `vacation` dates) SHALL spell a UTC offset `Z`, as pimalaya-cli's output contract does, and keep any other offset as found. A value a provider or a store spells (a Gmail or Graph passthrough, a pimdir date, the `Date:` header `imap fetch` prints in its envelope) SHALL keep its spelling.

#### Scenario: A message sent at UTC
- GIVEN a message whose header reads `Date: Mon, 01 Jan 2024 10:00:00 +0000`
- WHEN `himalaya --json message read` reads it
- THEN `headers.date` is `2024-01-01T10:00:00Z`, as `envelope list` prints its `date`

### Requirement: Gmail header selection applies to every format
`gmail messages get --header` and `gmail threads get --header` SHALL narrow the headers the text form renders whatever the requested format, and SHALL send Gmail's `metadataHeaders` under the metadata format, which Gmail honours there alone. The JSON SHALL stay the payload Gmail answered, its headers under `payload.headers`. Matching is case-insensitive, order and repeats are preserved, and passing no `--header` renders every header. Headers are read from the top-level payload part, where Gmail puts the RFC 5322 headers.

### Requirement: Raw message formats write bytes
A Gmail `get` command asked for the raw format SHALL decode the fetched message and write its RFC 5322 bytes through the shared byte writer rather than rendering a summary. This covers `messages get` and `drafts get`, and `msgraph messages get --raw`. The byte writer SHALL write the bytes verbatim in text mode and print `{data, size, mime}`, `data` in base64, under `--json`, so no command writes raw bytes to a JSON stdout; given a path, it SHALL write the file and print pimalaya-cli's `output::SavedOutput`, `{path, size}`, `Saved <size> bytes to <path>` in text. `gmail attachments get`, `msgraph attachments get` and `jmap email export` SHALL print through it.

### Requirement: One module per subcommand
A protocol command resource SHALL live in a sibling module file carrying its `pub mod` declarations and its `Command` enum, next to a folder holding one file per subcommand. Types live with the subcommand that owns them; a type serving several subcommands lives in the file of the one that owns it, or in its own module when it belongs to none.

### Requirement: Command types carry a domain and a target
A protocol command type SHALL be named `<Domain><Target><Verb>Command`, so a bare verb is never a type name. The domain prefix stays wherever the bare name would collide across the backends a CLI spans, and is otherwise omitted for the tables and value enums under the cli subtree.

### Requirement: Composers default the From header
`messages compose`, `messages reply` and `messages forward` SHALL fill the `From` header from the resolved account when `--from` is not passed: `email` as the address, `display-name` as the name it carries. An explicit `--from` SHALL win whole, the configured name never being grafted onto an address the user spelled out. With neither, the header SHALL be omitted rather than guessed.

The name SHALL be handed to the MIME builder apart from the address, so that a name carrying a comma, a quote or a non-ASCII character is encoded by the builder rather than by a quoting rule of Himalaya's own.

The composers SHALL write the `Date` (the current time at the local offset) and the `Message-ID` themselves, mime-rs's builder writing no field it is not given: the id sits on the domain of the `From` address when it is a plain dot-atom, else on the machine's hostname, else on `localhost`, its left part unique on the machine.

The same composers SHALL end the body with the account's `signature`, introduced by its `signature-delim`, when neither `--signature` nor `--signature-file` is passed. `--signature` SHALL win, and `--signature-file` SHALL win too, the configured signature standing down rather than shadowing the file the flag names.

With `--json` and neither `--save` nor `--send`, the same composers SHALL print the decoded `from`, `to`, `cc`, `bcc`, `subject`, `body` and `attachments` instead of the raw bytes, each address as the envelopes print one, `{name, email}` with `name` decoded or `null`, `from` the first `From:` mailbox or `null`, `to`, `cc` and `bcc` lists of them, groups flattened, and `subject` `null` without one, the body without any signature and with LF line breaks, `attachments` listing each attached file as `{filename, mime, size}` (the template carries no bytes: handing it back means passing the files again through `--attach`), so an editor can lay the message out and hand it back through the flags, sending appending the signature once. `--posting-style none` SHALL send the written body alone, the source left unquoted, for a writer who already laid the quote out.

The text body a composer writes SHALL end with a line break, so a printed single-part message ends on one, and a quoted empty line SHALL be `>` with no trailing blank. `--no-save` SHALL be refused without `--send`. An attached message (`message/rfc822`, from `--attach` or a forward's source) SHALL be written under 7bit, 8bit or binary only (RFC 2046 §5.2.1), its line breaks CRLF, or as the nested message it holds with its own parts encoded when 8-bit content cannot go as is.

### Requirement: Reply to all recipients
`message reply --all` (`-A`) SHALL keep the `To` of a plain reply and fill `Cc` with the `To` and `Cc` of the source, without the user's own addresses (the reply's `From`, `--from` or else the account's `email`; the account's `email`; the user names the account's IMAP and SMTP SASL blocks transmit when they hold an `@`) and without any address already in `To`, each address once, compared ignoring case, display names kept. An explicit `--to` SHALL replace the derived `To`, the `Cc` then being deduplicated against it, and an explicit `--cc` SHALL replace the derived `Cc` whole. Without `--all`, `Cc` SHALL stay empty unless `--cc` fills it.

#### Scenario: An account naming no email
- GIVEN an IMAP account setting no `email`, whose IMAP login is `me@icloud.com`, and a message to `me@icloud.com` and `bob@example.org`
- WHEN `himalaya message reply --all <id>` runs, or the same with `--from me@icloud.com`
- THEN the reply's `Cc` holds `bob@example.org` alone

### Requirement: Raw message input is shared
A command taking a raw RFC 5322 message SHALL resolve it through the shared `MessageArg`: its last positional, a file path or the message itself (after `--` when a word of it opens on a hyphen), options read wherever they stand ("An option after the words is an option"), else piped stdin read whole. A positional naming no file SHALL be the message only when it opens on a header field, a name of letters, digits, `-`, `_` and `.` then a colon, a one-letter name followed by `/` or `\` being a Windows drive, so a mistyped path, a Maildir file name holding `:2,` included, is refused rather than stored. The file SHALL be looked up under the positional as typed, before the `\n` and `\r` escapes of an inline message are decoded. Options naming flags SHALL take one value per occurrence, so the positional after them is never read as a flag. The message SHALL be kept as bytes, never decoded, so 8-bit content in any charset reaches the backend intact. The resolved message is normalised to CRLF and rejected when empty, so no backend receives a zero-length message.

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

### Requirement: Gmail draft writes return identities
`gmail drafts create` and `gmail drafts update` SHALL serialize the returned draft `id`, and nullable `messageId` and `threadId`, under `--json`, with schemas registered for both commands. Non-JSON output SHALL retain the existing success sentence. A response without a message SHALL still succeed, with `messageId` and `threadId` null, since the draft was already written.

### Requirement: IMAP fetch can return whole messages
`imap fetch --body` SHALL add `BODY.PEEK[]` (RFC 3501 §6.4.5) to the requested data items, so a caller downloads every message of a sequence set over one session instead of one `message read` per message. The fetch SHALL NOT set `\Seen`. The JSON output SHALL carry the message octets byte-exact as the standard Base64 of the `body` field, which the JSON Schema SHALL declare with `contentEncoding: base64` and `contentMediaType: message/rfc822`; the plain rendering SHALL print the size only. `--body` SHALL NOT imply `--envelope`.

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL NOT fail the command: the receipt SHALL say `sent: true` and `saved: false`, and list the failure in its `errors`, `id` `null`, coded by its cause, its `error` stating that the message was sent, and the command SHALL exit 3, so a caller never sends it twice. The receipts of `message send`, the composers and `message add` SHALL print `errors`, `[]` when nothing failed after the send. A backend filing every sent message itself (Gmail, under `SENT`) SHALL file the saved copy by labelling the message it sent with the mailbox and the flags, rather than add a second message, and SHALL report that message's id; pimdir files it through its queue. The save of a send SHALL be checked against `submit.copy`, the sending transport's, and a save alone against `message.add`. Graph files every message it sends in Sent Items itself and declares `submit.copy` `partial` with that detail: on a send, `--save` and `message.send.save-copy` SHALL be ignored, nothing added, and the receipt SHALL say `saved: true` with a `null` id. `--no-save` SHALL be refused there before connecting, coded `capability-missing` and naming the flag, Graph keeping the copy whatever is asked; a save without a send stays refused, `message.add` being `none` (backends: "Append gap").

#### Scenario: A copy the store refuses after the send
- GIVEN a Maildir account sending over SMTP
- WHEN `himalaya --json message send --save Nowhere <message>` runs and the server accepts the message
- THEN it prints `{"id":null,"saved":false,"sent":true,"queued":false,"errors":[{"id":null,"code":"mailbox-not-found","error":"Message sent, but saving a copy to Nowhere failed",…}]}` and exits 3

#### Scenario: A Graph send keeping no copy
- GIVEN an account whose storage backend is Graph
- WHEN `himalaya --json message send --no-save <message>` runs, or `message compose --send --no-save`
- THEN it fails coded `capability-missing`, naming `--no-save`, before the token is asked for

### Requirement: A sent reply or forward flags its source
`message reply --send` SHALL add the `answered` flag (RFC 3501 §2.3.2) to the source message once the send has succeeded, where the backend serves `flags.answered`; on one declaring it `none` (Gmail, Graph) the flag SHALL be skipped and the command succeed. A flag failing after a successful send SHALL NOT fail the command: the one receipt SHALL list it in its `errors`, `id` the source message, its `error` stating that the message was sent, and the command SHALL exit 3, printing nothing after the receipt. `message forward --send` SHALL add the `forwarded` flag (RFC 5788) to the source the same way, best-effort: a backend refusing the keyword SHALL be logged as a warning and SHALL NOT fail the command. A reply or forward only printed or saved SHALL leave the source untouched.

### Requirement: A forward carries the attachments of its source
`message forward` SHALL append every attachment part of the source (multipart containers excluded) after the quoted body, under its file name, with its MIME type, the parameters of its `Content-Type` (`charset`...) and its decoded bytes, the quoted body composing as it does without them. A nameless `message/rfc822` part SHALL be named from its subject, sanitized for the filesystem, plus `.eml`, or `attachment.eml`; any other nameless part SHALL be named `attachment` plus the extension of its MIME type (`.bin` for `application/octet-stream`), or bare `attachment`. `--no-attachments` SHALL carry none and combines with `--attach`. A reply SHALL never carry the attachments of its source.

### Requirement: A read message is a designed view
`message read --json` SHALL print `{id, etag, headers, text, html, parts}`, `id` the id the command took and `etag` `null`, a message's body never changing and no write taking a version to match, beside a view built from the raw message so it is the same on every backend, rather than the parser's own model; `message parse --json` SHALL print the same view without `id` and `etag`. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (groups flattened, an entry without a valid email dropped, `name` decoded or `null`), `subject` decoded, `date` as RFC 3339 with its offset, `Z` for UTC, or `null`, `messageId` without angle brackets, `inReplyTo` and `references` as lists without angle brackets, as envelopes print them, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found, decoded. `text` SHALL be every `text/plain` body part decoded to UTF-8 by its charset, lossily, a `format=flowed` part unwrapped (RFC 3676 §4.2), joined by a blank line, and `html` the first `text/html` body part, each `null` when absent. The message SHALL be parsed by mime-rs, whose structure is the one RFC 2046 delimits: a line opening with a boundary but carrying more than transport padding is no delimiter, an outer delimiter closes the parts nested in it, and a field whose name holds a blank (`Sub ject:`) is not the field it spells. The text output SHALL show the same headers, parts, sizes and names as the JSON view. Its From, To and Cc lines SHALL read the addresses as the JSON lists them, followed by each entry `headers.invalid` lists as `(unreadable: <entry>)`, a header holding neither being left out. `--raw` SHALL write the message bytes verbatim, and print `{data, size, mime}` under `--json`.

### Requirement: Malformed mail reads as its sender meant
The view SHALL read a header from its raw bytes: raw 8-bit bytes SHALL be decoded as UTF-8 when they are valid UTF-8 (RFC 6532) and as Windows-1252 otherwise, and adjacent RFC 2047 encoded words of one charset, separated by linear whitespace alone, SHALL be joined as bytes before their charset is decoded, the whitespace between adjacent encoded words being dropped (RFC 2047 §6.2). This applies to the subject, the display names, the list headers and the file names; an RFC 2231 file name SHALL win over a plain one. A text part labelled `us-ascii` or `ascii`, or naming no charset, SHALL be read as UTF-8 when its bytes are valid UTF-8 and as Windows-1252 otherwise, in `text` and `html` alike. The charsets are the WHATWG Encoding Standard's: UTF-7 (RFC 2152), outside it, SHALL be read as written. A `Content-Type` whose type or subtype is missing, empty or not an RFC 2045 token SHALL be taken as `text/plain; charset=us-ascii` (RFC 2045 §5.2), its part reporting that `mime` and `charset`.

### Requirement: A wrong weekday keeps its date
An envelope SHALL read a `Date:` whose day-of-week disagrees with its date (`Mon, 15 Nov 2026`, a Sunday) as the date it names, on every backend: IMAP from its `ENVELOPE` and Gmail from its metadata SHALL parse it again without the day-of-week when chrono refuses it, as Maildir, mbox and the read view read it.

#### Scenario: A Gmail message dated with a wrong weekday
- GIVEN a Gmail message whose `Date:` is `Mon, 15 Nov 2026 11:00:00 +0000`
- WHEN `himalaya --json envelope list` lists it
- THEN its `date` is `2026-11-15T11:00:00Z`

### Requirement: The read view lists unreadable address entries
`headers.invalid` SHALL be an object `{to, cc, bcc, replyTo, from, sender}`, each a list of the entries of that header that are not readable as an address and so are absent from its list, as decoded text (encoded words decoded, trimmed), in header order, empty when none. A group name SHALL NOT be an entry, nor an empty group. `message read --json` and `message parse --json` SHALL print the same.

### Requirement: Every part has an id and a role
`parts` SHALL list every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`, the `id` being the one `attachment list` and `attachment download` take: the 1-based position of the part in the whole part list. A `multipart/*` SHALL be no part; `message/rfc822` or `Content-Disposition: attachment` SHALL be an `attachment`; a `text/plain` or `text/html` without a filename SHALL be a `body`; an `image/*` marked inline or carrying a `Content-ID` SHALL be `inline`; anything else SHALL be an `attachment`. `filename` SHALL be RFC 2231 and RFC 2047 decoded, `null` when the part names none; `size` SHALL count the part's bytes once its transfer encoding is undone, in its own charset; `contentId` SHALL carry no angle brackets; `method` SHALL be a `text/calendar` part's `method` parameter, uppercased. `attachment list --json` SHALL report the same ids, sizes, `contentId` and `role`, and `path`, where `attachment download` wrote the part, `null` until then.

### Requirement: A message too complex is refused whole
A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines (folded lines and the headers of every part and attached message included), SHALL be refused with the JSON error code `message-too-complex` rather than read in part, by `message read` (except `--raw`) and the `attachment` commands, as SHALL a message the MIME parser (mime-rs) stopped splitting at its own entity or nesting cap, a part it left unsplit hiding what it holds.

### Requirement: A part can be read without touching the disk
`attachment download <MESSAGE-ID> <PART-ID> --stdout` SHALL take exactly one part id, any leaf part `message read` lists, and write that part's decoded bytes to stdout with nothing else, touching no file; `--dir` SHALL be refused beside it. Under `--json` it SHALL print `{id, mime, filename, size, data}`, `data` in base64. Without `--stdout`, the parts named SHALL be written to the download directory as the same decoded bytes; given no id, the parts `attachment list` lists, the inline ones only with `--inline` (`-i` a hidden alias of it, `-i` meaning `--interactive` across the family).

### Requirement: A message can be read without an account
`message parse [EML]` SHALL read a raw message from a path, or from stdin with `-` or when omitted, and SHALL print what `message read` prints for the same bytes, without the `id` and `etag` a read carries. It SHALL resolve no account and read no configuration. A source holding nothing but whitespace SHALL be refused, naming where it was read from; the bounds and the `message-too-complex` code SHALL apply as on `message read`.

### Requirement: A parsed message hands one part over
`message parse --part <PART-ID>` SHALL write that part's bytes, transfer encoding undone, to stdout, or under `--json` `{id, mime, filename, size, data}` with `data` in base64; the ids SHALL be those of the view. An unknown id or a container's SHALL be refused.

`message parse --section <SECTION>` SHALL do the same for the leaf part at that IMAP section (RFC 3501 §6.4.5): `1` for the body of a message that is not multipart, else each part's 1-based position among its siblings, joined with dots down the message's own multiparts. The JSON `id` SHALL stay the view's. A container's section, a part of an attached message, or a section naming nothing SHALL be refused; `--part` and `--section` SHALL NOT be given together.

### Requirement: Downloaded attachments never replace an existing entry
Attachment downloads SHALL atomically create a new file, refusing to follow an existing leaf symlink. An occupied name SHALL be suffixed with a number before its extension. If the original name and suffixes 1 through 1023 are occupied, the command SHALL fail rather than overwrite a file. Concurrent downloads SHALL reserve distinct names.

### Requirement: A command given message ids says how many it found
`flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL require ids, a repeated id being acted on and counted once. An id the mailbox does not hold SHALL be an `errors` entry coded `message-not-found`, the others acted on; when none exists, nothing SHALL change and the command SHALL print its result with `count` 0 and one entry per id, exiting 3. They SHALL report how many were acted on as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `raw` spelled as the backend writes it (`$flagged` on JMAP), `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `pimdir flag add|set|remove` SHALL fail, exit code 1, with the code `message-not-found` when none of the given ids exists, changing nothing. `message delete` in the trash SHALL count the ids the trash holds and remove those alone. On a backend whose ids are account-wide (JMAP, Gmail, Microsoft Graph), an id SHALL count as found only when its message is in the given mailbox. A move whose source and destination are one mailbox SHALL change nothing, on every backend, counting the ids the mailbox holds: IMAP sends no `MOVE` nor `COPY`, the UIDs kept, mbox leaves the file as it was, the raw `mbox message move` printing the ids given, and pimdir stages nothing. On Maildir and mbox every id SHALL be looked up before the first write, an unknown one an entry, a removal from the trash included. The protocol commands of the local stores taking ids (`maildir` and `mbox` `message copy|move`, `maildir` and `mbox` `flag add|set|remove`) SHALL require them too and look every id up before the first write, an unknown one refusing the whole set and nothing changed: naming every unknown id under `message-not-found`; the flag commands SHALL print each id once. A `maildir message move` whose source and target are one Maildir, without `--subdir`, SHALL change nothing, the ids kept. `maildir message copy|move` and `mbox message copy|move` SHALL print the ids the messages have in the target, each message once. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya --json flag add --flag flagged 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":0,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 3 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false,"errors":[{"id":"99999999","code":"message-not-found",…}]}`

#### Scenario: An mbox move onto its own file
- GIVEN an mbox mailbox `A` holding two messages
- WHEN `himalaya --json message move -f A -t A <id>` runs, then `himalaya --json mbox message move -m A -t A <id>`
- THEN the first prints `count` 1, the second `{"ids":["<id>"]}`, both exit 0, and the file is byte for byte what it was

### Requirement: A queued write says it is queued
On a backend that queues its writes for a sync engine to apply later (pimdir), the receipt of a unified write SHALL carry `queued: true`, and its text form SHALL say the write is queued rather than done: `message send`, `message add`, a composer's save or send, `message delete`, `message move`, `message copy`, `flag add|set|remove` and `mailbox create|rename|delete`. A mailbox write SHALL print its `id` `null` while queued, and no queue detail (a row, a queue id) SHALL reach a unified output. Every other backend SHALL print `queued: false`.

#### Scenario: A send on a pimdir account
- GIVEN a pimdir account with `mailbox.alias.sent` set
- WHEN `message send` runs
- THEN it prints `Message queued for sending`, not that the message was sent

#### Scenario: A delete on a pimdir account
- GIVEN a pimdir account with a trash mailbox
- WHEN `message delete 3` runs on another mailbox
- THEN it prints `Queued the move of 1 message(s) to the trash`

### Requirement: Unified mailbox writes
`mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST "" ""` reports, `/` on Gmail, Graph and the local stores. NAME SHALL be refused when empty, blank or holding a control character, and per backend when it holds the hierarchy delimiter (IMAP's, `/` on Gmail, Maildir and the unified mbox writes), or names no listable level (Maildir `cur`, `new`, `tmp`, a leading dot on Maildir, a dot on Maildir++, the names the mbox backend refuses). Creating an existing mailbox and renaming onto one SHALL be refused. A create naming a mailbox that exists SHALL fail coded `mailbox-duplicate`, exiting 1 with nothing created nor staged, wherever the backend can tell: a Maildir folder already at the path (a file there being no mailbox, refused uncoded), an mbox file already there (a directory refused uncoded), a mailbox a pimdir store lists or has its creation queued (backends, "pimdir creates a mailbox by queueing an intent"), an IMAP `CREATE` the server refuses with `ALREADYEXISTS` (RFC 5530 section 3), or refuses with no code while `LIST` then shows the mailbox, a JMAP `Mailbox/set` refused while a mailbox of that name sits under the same parent (siblings sharing no name, RFC 8621 section 2) and coded no other way, a Gmail label create answered 409, and a Graph folder Graph answers exists (409 `ErrorFolderExists`) at the top, or one of that name the parent holds, hidden ones included, looked up before the folder is created at the top. A Graph namesake at the top of a folder asked under a parent is no duplicate, its refusal kept uncoded. Every such refusal SHALL read ``Cannot create mailbox `NAME`: it exists already``, NAME the mailbox as the create resolved it (its path under the parent on IMAP and the local stores), what the backend or the store answered (the server's refusal, the pimdir id listing it, its queued creation, the Graph parent holding it) kept in `sources`, as Cardamum and Calendula word a duplicate address book or calendar. The protocol namespaces' own creates (`imap mailbox create`, `jmap mailbox create`, `gmail labels create`, `msgraph mail-folders create`) keep the server's refusal as it is; `maildir create` and `mbox create` code the duplicate as the unified create does. The root of a local store (the root Maildir, however named, and the mbox spool) SHALL never be deleted or renamed. Deleting a local mailbox that holds others SHALL be refused, naming them, and a Maildir holding any directory besides `cur`, `new` and `tmp`, listed or not, SHALL be refused too, naming it, the removal taking it along otherwise; on IMAP a server's refusal of the same SHALL name them too, and a missing mailbox SHALL read ``IMAP mailbox `X` not found`` for rename and delete alike. On IMAP a `--parent` the server marks `\Noinferiors` (RFC 3501 section 7.2.2) SHALL be refused before the `CREATE`, ``IMAP mailbox `X` can hold no other (\Noinferiors)``. On Gmail, where a label nests by name, a rename SHALL also rename every sublabel, as IMAP `RENAME` moves the inferiors; a delete SHALL leave the sublabels under their full names.

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

### Requirement: The important and junk flags are in the unified API
The shared flag arguments SHALL include `important` (`$Important`, RFC 8457) and `junk` (`$Junk`), each its capability, `flags.important` and `flags.junk`.

### Requirement: The auto-reply is in the unified API
`vacation get` SHALL print the account's auto-reply as `{enabled, start, end, subject, text, html}`, `start` and `end` the window it answers in, RFC 3339 in UTC, `null` for no bound (`Start:` and `End:` in text), and the default one, off with nothing set, when the backend holds none. `vacation set` SHALL change only what it is given (`--enable` or `--disable`, `--from`, `--to` as RFC 3339 or a `YYYY-MM-DD` read as midnight UTC, `--subject`, `--text`, `--html`, each cleared by its `--no-from`, `--no-to`, `--no-subject`, `--no-text`, `--no-html`, which conflicts with its setter) and print the auto-reply as it then stands; given none of these options, it SHALL fail as a usage error before connecting. JMAP serves it when its session advertises `urn:ietf:params:jmap:vacationresponse` (RFC 8621 section 8), Gmail through `settings.getVacation` and `updateVacation`, its dates epoch milliseconds, the settings read first so the Gmail-only ones survive.

### Requirement: An account lists what it serves
`account capability` SHALL print pimalaya-cli's `capability::AccountCapabilityOutput`, one row per unified capability, `{name, backend, level, detail}` under `{account, capabilities}`: the backend the resolver picks for the account and `--backend`, `null` when none is configured, and that backend's level and detail. It SHALL read the configuration and the capability tables and connect nowhere. On a pimdir account, a write SHALL be read from what the store's sources declare: an item write through the store's gate in each of the account's mail collections (`none` refused in all, `partial` refused in some or passing in part), an intent served when a source performs it, the static row kept when no source declares anything.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `account capability` runs
- THEN `mailbox.list` names `maildir`, `vacation` names `jmap` at `server`, and nothing connects

### Requirement: A write prints what it acted on
Every write SHALL print what it acted on, never a sentence alone. A unified write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message add` `{id, sent, queued}`, `id` the added message's and `sent` whether `--send` sent it too, `message send` and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. `message delete` SHALL print the receipt of a batch delete, `{action, count, queued, errors}`: it takes several ids, so it counts the messages it acted on as `message copy` and `message move` do, where a delete of one item names it (`{id, queued}`), and `action` says what became of them, `moved-to-trash`, `deleted` from the trash, or `flagged` `\Deleted` with an expunge owing. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Absent is null
An output type of Himalaya's own SHALL print every key, `null` when the value is absent, so its schema marks every key required. A message without a `Subject:` SHALL print `subject` as `null`, not as an empty string, in every envelope, search hit, thread entry (`imap thread`, `pimdir thread`), queued row and `pimdir tag` row. A provider payload printed as received SHALL keep its own omissions.

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

### Requirement: An envelope listing says how many messages the mailbox holds
`envelope list` SHALL print `total` beside `envelopes`, `nextPage` and `errors`: how many messages the mailbox holds, when the backend answers it within the listing's own exchanges (the `EXISTS` of IMAP's `SELECT`, JMAP's `calculateTotal`, the entries Maildir, mbox and pimdir list), and `null` otherwise, Gmail among them. No backend SHALL send a request for the total alone. `envelope search`, sharing that output, SHALL print `total` as `null`, a search counting its matches rather than the mailbox. The plain text output does not change.

#### Scenario: An IMAP mailbox of 120 messages
- GIVEN an IMAP mailbox holding 120 messages
- WHEN `himalaya --json envelope list -s 25` runs
- THEN it prints 25 envelopes, `nextPage` `"2"` and `total` 120

#### Scenario: Gmail
- GIVEN a Gmail account
- WHEN `himalaya --json envelope list` runs
- THEN `total` is `null`

### Requirement: An IMAP command prints its completion
Every `imap` command but `raw` SHALL print the tagged completion of its exchange: `status: "OK"` beside what the command understood of the server's answer, or `{status, text}` when the server answers `NO` or `BAD`, exiting 3 after it. A command getting no completion (a network, TLS or parse failure, a local error) SHALL fail with the failure object. `status` SHALL request and print the RFC 3501 items alone (`messages`, `recent`, `unseen`, `uidNext`, `uidValidity`), each `null` when the server leaves it out, and no key it never requests. `select` SHALL print the mailbox's state (`flags`, `exists`, `recent`, `firstUnseen`, the number of the first unseen message, `permanentFlags`, `uidNext`, `uidValidity`, `highestModSeq`), `expunge` the expunged message numbers, `store` the FETCH data the server returns, `append` the `APPENDUID` it returns (UIDPLUS), `copy` and `move` the `COPYUID` (`uidValidity`, and `sourceUids` beside `destinationUids`, each source UID at the place of the UID it got), `null` and empty without UIDPLUS or when it counts over a million messages, the mailbox writes `{mailbox}` and `close` and `unselect` the status alone. `fetch` SHALL print envelope addresses as `{name, email}`, and `search` its ids as numbers, as `sort` does. io-imap returns a typed exchange's output whole, so these commands print no events. `thread` SHALL be refused with `capability-missing` before anything is sent when the server does not advertise `THREAD=` the algorithm.

#### Scenario: A mailbox that does not exist
- GIVEN an IMAP account
- WHEN `himalaya --json imap select Nope` runs
- THEN it prints `{"status":"NO","text":"…"}` with the server's text and exits 3

### Requirement: A local store flag prints its meaning beside its spelling
Every flag Himalaya prints SHALL be the one `Flag` noun, `{raw, iana}`, `raw` the spelling the store keeps and `iana` the IANA keyword it stands for, `null` for one none registers: an envelope's `flags`, `pimdir flag list` (its `flags` `null` when the sync engine never read them), the flags of a `pimdir tag` row, and the rows of `maildir flag list` and `mbox flag list`, `raw` there being the store's letter and `iana` `null` for mbox's `O`, which has none; mbox rows SHALL also name the `header` carrying the letter.

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

### Requirement: An item that failed prints last
In the text output, the `errors` entries SHALL print last, as pimalaya-cli's `error::ItemErrors` prints them: after a list's items or a write's sentence, one blank line, then one `Error:` line per entry, in order, its cells the id, `-` when null, the code, `-` when null, then the message, aligned among the entries as columns are. Nothing SHALL print, blank line included, when there is no entry.

#### Scenario: One unknown id of a write
- GIVEN a mailbox holding message `42` and no message `99`
- WHEN `himalaya flag add --flag flagged 42 99` runs
- THEN it prints `Successfully added flags to 1 message(s): \Flagged`, a blank line, then `Error:  99  message-not-found  …`, and the exit status is 3

#### Scenario: An unreadable envelope
- GIVEN an IMAP page holding envelope `4` and a FETCH line naming no UID that does not decode
- WHEN `himalaya envelope list` prints it
- THEN it prints the fields, a blank line, the line of `4`, a blank line, then `Error:  -  message-unreadable  …`

### Requirement: A protocol refusal is a result
Inside the `gmail`, `msgraph`, `jmap`, `sieve` and `smtp` namespaces, a completion the server refuses SHALL print as the command's result and exit 3, as `imap` and the `raw` commands do (pimalaya-cli cairn/spec/output.md, *A protocol refusal is a result*); a command getting no completion (a network, TLS or decoding failure, a local error, a value refused before sending) SHALL print the failure object and exit 1. An HTTP 4xx or 5xx a `gmail`, `msgraph` or `jmap` command meets (the `request` commands included), a JMAP request refused whole (RFC 8620 §3.6.1), a JMAP method error (§3.6.2) and the SetError refusing the one write of a `jmap` command (§5.3) SHALL print `{refusal: {status, reason, message, code, retryAfter}}`: `status` the HTTP status, `null` for a refusal inside a 2xx; `reason` the provider's name for it in its spelling (Gmail's first reason, else its canonical status; Graph's `error.code`; the JMAP error type, the problem type's URN namespace stripped), `null` when it names none; `message` the provider's text, the type standing in when a JMAP error carries none; `code` the code the failure would carry (*A server's refusal codes the whole command*), `null` when none applies; `retryAfter` the seconds the server asked to wait, `null` when it asked none. A typed `sieve` command answered `NO` or `BYE` SHALL print the completion `{status, code, text}` as `sieve raw` prints it. `smtp send` refused SHALL print `{refusals: [{rcptTo, code, text}]}`, one reply per refusal in the order received (a refused `MAIL FROM` or `DATA` with `rcptTo` `null`, one per recipient when the server refused them all), the message reaching nobody. Each command's JSON Schema SHALL describe its own result and the refusal as two alternatives told apart by their keys.

#### Scenario: A Gmail id that does not exist
- GIVEN a Gmail account answering 404 `notFound`
- WHEN `himalaya --json gmail request get users/me/labels/nope` runs
- THEN it prints `{"refusal":{"status":404,"reason":"notFound","message":"Requested entity was not found.","code":null,"retryAfter":null}}` and exits 3

#### Scenario: A recipient the server refuses
- GIVEN an SMTP server answering `RCPT TO:<refused@example.org>` with `550 5.1.1 No such user`
- WHEN `himalaya --json smtp send -f t@example.org -t refused@example.org <message>` runs
- THEN it prints `{"refusals":[{"rcptTo":"<refused@example.org>","code":550,"text":"5.1.1 No such user"}]}` and exits 3

### Requirement: A value a protocol command refuses is refused with the command line
A value of a protocol command's arguments the command would refuse whatever the server answers SHALL be refused with the command line, coded `usage-invalid`, exit 2, before the configuration is read, the secret command run or the session opened: the sequence set of `imap fetch`, `store`, `copy` and `move`; `--mail-from` without `--rcpt-to`, or the reverse, on `jmap submission create`; `msgraph messages update` naming no change; the method and an off-origin URL of `gmail request` and `msgraph request`; and `never` beside another condition in `smtp send --notify`. A value read from a file or stdin (a raw command, a JMAP request body, a Sieve script) SHALL be read and checked inside the command, a session refusing stdin as `stdin-taken`.

#### Scenario: A sequence set that is none
- GIVEN an IMAP account whose password is a command
- WHEN `himalaya --json imap fetch -m INBOX abc` runs
- THEN it exits 2 with the failure object coded `usage-invalid`, and the password command did not run
