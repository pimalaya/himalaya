---
cairn: delta
change: m2dir-is-dropped
---

# Delta

## ADDED Requirements

### Requirement: An m2dir block is named, not loaded
(config) Himalaya SHALL NOT read m2dir stores. An account carrying an `m2dir` block, whatever it holds, SHALL load as if the block were absent, so a file shared with himalaya-tui or written for v2 keeps loading. When no configured backend matches `--backend`, that error SHALL name the `m2dir` block as ignored since v3 removed m2dir and point at MIGRATION.md. `m2dir` SHALL NOT be a `--backend` value, clap refusing it as any unknown value.

#### Scenario: An m2dir-only account
- GIVEN an account whose only storage block is `m2dir.root = "~/Mail"`
- WHEN `envelope list` runs against it
- THEN the configuration loads and the command fails naming the account, the ignored `m2dir` block and MIGRATION.md

## MODIFIED Requirements

Each requirement below drops m2dir from its backend lists; its text is otherwise unchanged. The commands overview drops m2dir from the protocol-specific APIs too.

### Requirement: Backends reading the MIME structure mark attachments by the pimdir rule
(backends) IMAP, Maildir and mbox SHALL answer `hasAttachment` by the rule of the pimdir mark (STORAGE Annex A.1) over the leaf parts of a message. A part SHALL mark the message when its `Content-Disposition` is any but `inline`. A part stating none SHALL mark it unless it is text, multipart or named by a `Content-ID`. An RFC 1847 protocol part SHALL never mark it: any child of a `multipart/encrypted`, any child but the first of a `multipart/signed`. A `Content-Type` that is no pair of RFC 2045 tokens SHALL read as `text/plain`. A `message/rfc822` part is a leaf, the parts of the attached message being its own. The `attachment` search clause SHALL read the same mark on the backends that search locally. JMAP and Graph SHALL relay what the server says; Gmail SHALL leave it `null`.

`attachment list` SHALL stay wider than this mark: it also lists signatures and images named by a `Content-ID`, and some text parts.

#### Scenario: A PDF without a disposition
- WHEN a message holds a single `application/pdf` part with no `Content-Disposition`
- THEN `envelope list --has-attachment` reports `hasAttachment: true` on IMAP, Maildir and mbox

#### Scenario: An inline PDF
- WHEN a `multipart/mixed` message holds a text part and an `application/pdf` part with `Content-Disposition: inline`
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir and mbox

#### Scenario: A signed message
- WHEN a `multipart/signed` message holds a text part and its signature, marked `attachment` or not
- THEN `envelope list --has-attachment` reports `hasAttachment: false` on IMAP, Maildir and mbox

### Requirement: The envelope carries its threading pointers
(backends) The shared `Envelope` SHALL carry `message_id` and `in_reply_to`, the RFC 5322 §3.6.4 identity of a message and of the message(s) it replies to, so a client can pair a reply with its parent from a listing rather than by reading bodies.

`in_reply_to` SHALL be a list, the grammar being `1*msg-id`, and every id in it SHALL be normalised exactly as `message_id` is (angle brackets and surrounding whitespace stripped), so the two compare byte-for-byte whatever backend surfaced them.

Each backend SHALL source the field from the response its listing already makes, and SHALL leave it empty rather than issue a request of its own: IMAP from the `ENVELOPE` (RFC 3501 §7.4.2, 9th element), JMAP from the `inReplyTo` property of `Email/get`, Gmail from the metadata headers, Maildir and mbox from the parsed message, and pimdir from the stored summary. Graph leaves it empty, `In-Reply-To` living in `internetMessageHeaders`, which a listing selection does not return.

The field SHALL NOT be printed by the `envelope list` text output, where raw msg-ids would be noise; it rides the JSON output.

### Requirement: Local storage backends
(backends) Maildir and pimdir SHALL adapt io-maildir and io-pimdir. Maildir stores added messages under `cur/` and SHALL read an entry's flags through io-maildir rather than parsing the filename itself, so the meaning of a Maildir name is decided in the library that owns the format. A Maildir store listing skips plain files beside its folders; a missing root fails, naming it. `maildir message save` SHALL refuse flags with `--subdir new`, a Maildir keeping none there. pimdir is an offline cache the sync engine populates, io-pimdir implementing both the store and the engine: reads project the store's items with their typed mail summaries (pimdir STORAGE Annex A) without body reads, and writes are actions queued for the store's owner to apply and a later sync to propagate, never direct SQL.

### Requirement: Sending transport
(backends) Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API, and pimdir through its queue. Storage backends that cannot send (IMAP, Maildir, mbox) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 3.6.3), which io-smtp removes after the envelope is derived. The protocol-level `smtp send` SHALL transmit its message verbatim, its envelope being explicit. Its `--notify`, `--ret` and `--envid` SHALL request delivery status notifications (RFC 3461), `NOTIFY` going on every `RCPT TO` and `RET` and `ENVID` on `MAIL FROM`; the command SHALL fail before `MAIL FROM` when the server does not announce `DSN`, and SHALL refuse `never` combined with another condition and an `ENVID` needing xtext encoding or longer than 100 characters. The unified `message send` requests none.

### Requirement: Mailbox role
(backends) A shared mailbox SHALL carry an optional role (inbox, all, archive, drafts, flagged, important, junk, sent, subscribed, trash, or a verbatim unknown one), shown by `mailbox list` in its text and JSON output. JMAP reads it from the mailbox `role`, Gmail from its fixed system-label ids, Microsoft Graph from its well-known folder names resolved to folder ids through `$batch` ("Graph batches four requests at a time"), a name Graph answers 404 for being skipped and any other failure failing the command, pimdir from the role its sync engine recorded from the server (`collections.role`), and IMAP from the special-use attributes its plain `LIST` carries besides `INBOX` (provider-quirks), as mbox marks its spool when one is configured. Maildir has no native role. On IMAP, JMAP, Gmail, Graph and pimdir a role name addresses the mailbox carrying it. A `mailbox.alias.<role>` entry overrides the native role of the mailbox it names, and `message delete` takes its trash from that alias before the backend's trash role; with neither, it SHALL fail with `capability-missing` before anything is moved. On IMAP, JMAP, Gmail and Graph a name several mailboxes carry SHALL be refused, naming their ids, and on JMAP and Gmail a value naming no mailbox by id, name or role SHALL be refused rather than sent to the server, as on Graph unless it may name a folder Graph does not list ("Graph folders are named by path").

### Requirement: Mailbox counts are asked for
(backends) `mailbox list` SHALL fill a mailbox's `total` and `unread` with `--counts` only, both `null` otherwise, Graph returning them inline included. With `--counts`, IMAP SHALL issue one `STATUS` per mailbox, JMAP read them from `Mailbox/get`, Gmail issue one `labels.get` per label, Graph read them from the folder, mbox sync the index of each file, and pimdir count the items and the unread mail of the store. Maildir SHALL leave both `null`.

### Requirement: Every backend builds alone
(backends) `cargo clippy --all-targets -- -D warnings` SHALL pass with the default features; with `--all-features`; with `--no-default-features` and a TLS provider alone (`rustls-ring`); with a TLS provider and each backend feature alone (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `sieve`, `pimdir`); with `native-tls,jmap`; and with the three local backends together (`maildir,mbox,pimdir`). Code only some backends use SHALL be gated on the item with exactly those features, never allowed crate-wide. A configuration type only some backends read SHALL stay compiled in every build and be allowed to go unread in the builds without them, so every build parses and checks the same configuration, the blocks of the backends it leaves out included.

#### Scenario: A Maildir-only build
- GIVEN `--no-default-features --features rustls-ring,maildir`
- WHEN `cargo clippy --all-targets -- -D warnings` runs
- THEN it passes, where it reported the proxy, TLS, SASL and IMAP options of the configuration unread

#### Scenario: A block for a compiled-out backend
- GIVEN a Maildir-only build and an account whose `imap.sort` block carries an unknown key
- WHEN `account list` reads the configuration
- THEN it is refused naming that key, as a full build refuses it

### Requirement: Unified commands over a resolved transport
(commands) The unified commands SHALL run over an `EmailClient` owning one backend-client variant per compiled-in backend, each verb resolved to a configured transport. The verbs exchanging ids resolve to the storage backend: the first configured one the global `--backend` flag allows, local before network (Maildir, mbox, pimdir, JMAP, Gmail, Graph, IMAP). Sending resolves to the storage backend when it sends (JMAP, Gmail, Graph, pimdir), else to the SMTP transport, which SHALL connect for each send and close after it. `--backend` SHALL filter the resolution. Each shared method matches the resolved backend and calls its per-protocol adapter.

### Requirement: account check covers every declared backend
(commands) `account check` SHALL check every backend the account declares and `--backend` allows, and print the report whole, one row `{backend, ok, code, error}` per backend, `code` and `error` `null` when the backend answered. When any fails, it SHALL exit 2 after the report, so a script reads the outcome from the exit code and each backend's code from its row. A pimdir block SHALL be checked by opening the store as a reader, listing its mailboxes, then opening a producer and probing it (`PimdirProducer::probe`), which takes and releases the staging lock without waiting and the database's write lock, writing nothing into the store, so an account declaring only a pimdir block is checked rather than refused as declaring no backend, and a store a write cannot reach fails the check rather than the first write: a collector holding the staging lock fails it at once with `store-busy`, and the check leaves nothing in objects/. A Maildir block SHALL be checked by listing its store, a missing root failing the check.

### Requirement: An account list names every block
(commands) `account list` SHALL print one row per account, `{name, default, backends}`, sorted by name, `backends` naming each block the account declares among `imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`, `smtp` and `sieve`, in that order, whichever backends the build compiles. It SHALL load the configuration without taking an account.

#### Scenario: A pimdir store
- GIVEN an account declaring a `pimdir` block and an `smtp` block
- WHEN `account list` runs
- THEN its BACKENDS cell reads `pimdir,smtp`

### Requirement: Text output is plain text
(commands) A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

Every list SHALL print through pimalaya-cli's `Columns` with no header row, one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-` and a line break inside a cell as a space. Its short cells SHALL read first and its free text (a subject, a name, a path, an error) last; a cell holding several values SHALL join them with `,`, a size SHALL print with no blank (`1KiB`), a mailbox's unread and total counts SHALL print as one cell, unread first (`3/120`), a count the backend gave none of as `-`, and a yes-or-no cell SHALL print its word (`default`, `active`, `permanent`, `inline`, `subscribed`) or nothing, so a line splits on blanks.

Above the items SHALL print `Account:` (the account name, then in parentheses the backend answering the whole list) and the fields naming what is listed (`Mailbox:`, `Message:`, `Query:` and the others below). After them, a paged list SHALL print `Page:` and, while a page follows, `Next page:` with the flag reaching it: a list paged by number names the page and the items it holds (`2, envelopes 26 to 50`, followed by `, the last` when no page follows, or `empty`; `all` for an unpaged listing) and gives `--page`; a list paged by token counts them (`25 messages`, `, the last`, `empty`) and gives `--page-token`. The other fields placing the answer (`Incomplete:`, `History id:`, `Cancel:`) SHALL follow in the same block, and a blank line SHALL part the block from the items, nothing printing under them. Field values SHALL start in one column on every list, two spaces after `Next page:`. A list reading no account (`account list`, `maildir flag list`, `mbox flag list`) SHALL print its items alone; a list without items, its header alone, unless it says otherwise below.

A list of messages SHALL print each the way `mscan` prints a mailbox: its id; four flag columns, `*` for no `\Seen`, `R` for answered, `!` for flagged and `@` only when the backend says the message has an attachment, each a space otherwise; its date as `envelope.list.datetime-fmt` (`%F %R` by default) and `envelope.list.datetime-local-tz` (true by default) configure it; the list's own short cells; its senders' display names, the address standing for a nameless one, joined with `, `; then its subject, last. These lists print so:

- `envelope list|search`: the envelopes in the listing's order; the senders, or `to ` and the recipients with `--recipient`. The size SHALL NOT print. Above: `Mailbox:` (its alias resolved) and, for a search given a query, `Query:` (the query as parsed). Paged by number.
- `jmap email query|get`: the date is `receivedAt`. Above: `Mailbox:` when `--mailbox` filters. A query is paged by number, a page following while one comes back full.
- `msgraph messages list`: the date is `sentDateTime`, else `receivedDateTime`. Above, `Folder:`, `Search:` and `Filter:` when given, and `Count:` with `--count`. Paged by token, a next link that is no token printed as `Next link:`.
- `pimdir search`: its mailboxes, joined with `,`, then, when a text term matched any hit, `body` or `headers`; the snippet SHALL NOT print. Above: `Query:`. Paged by number; after the page, `Incomplete:` and what the answer could not see, when it is not whole.
- `imap thread`: each thread from its first message, the subject indented two spaces per level: a run of members nests each under the one before, the answers sit under the last, and the answers of a message the mailbox lacks stand side by side at its level. A message whose envelope could not be fetched prints its id, its other cells empty. Above: `Mailbox:` unless `--no-select`.
- `pimdir thread`: each message after the earlier one its `In-Reply-To` names, its subject indented one level further, one whose parent the account lacks standing at the margin; its mailboxes, joined with `,`, before the senders. Above: `Thread:` when a thread holds the message.

`pimdir tag` and `pimdir queue list` SHALL print the same shape with what they carry. `pimdir tag`: the message id; the flags once the tag applies, blank where they were never read; the status (`planned`, `queued`, `unchanged`, `flags-unknown`, `several-sources`, `not-synced`, `unsupported`); the mailbox; the queue row; the subject. Above: `Query:` and `Incomplete:` as for a search. `pimdir queue list`: the row; `queued` or `parked`; the staged flags, blank for a parked row; when it was queued; the action (`save` or `send`, a parked row's kind); `to ` and the recipients' names; then the subject, or why the row was parked. Above: `Mailbox:` and `Cancel:` with the command cancelling a row, when any is listed.

The other lists SHALL print these cells, in this order:

- `mailbox list`: the id, the role, with `--counts` the counts, the name.
- `attachment list|download`: the part id, `inline`, the size, the type, then the filename, or the path once written. Above: `Mailbox:` and `Message:`. Without any part to list, `No attachments` in place of the items.
- `account list`: the name, the backends, `default`. `account capability`: the capability, the backend, the level, the detail; `Account:` names no backend.
- `sieve list`: `active`, the name.
- `imap list`: the delimiter, the role, the attributes, the name. `imap flag`: the flag, `permanent`; above, `Mailbox:`. `imap search|sort`: one id per line; above, `Mailbox:` unless `--no-select`.
- `maildir list`, `mbox list`: the name, the path. `maildir flag list`: the letter, the name, in the letters' ASCII order (`D`, `F`, `P`, `R`, `S`, `T`), as `-f` lists its values; `mbox flag list`: the letter, the header, the name.
- `jmap mailbox query|get`: the id, the role, the counts, `subscribed`, the name; a query paged by number. `jmap submission query|get|create`: the id, the email id, the identity id, the undo status, the send date; a query paged by number. `jmap identity get`: the id, the email, the name. `jmap thread get`: one email per line, beside its thread.
- `gmail messages list`: the id, the thread id. `gmail threads list`: the id, the history id, the snippet. Both above: `Labels:` and `Query:` when given. `gmail drafts list`: the draft id, the message id, the thread id; above, `Query:`. The three paged by token. `gmail labels list|get`: the id, the type, the counts, the name. `gmail history list`: one line per message change, the record id, `added`, `deleted`, `labeled` or `unlabeled`, the message id, then the labels; above, `Since:` and `Label:`; paged by token, counting records; after the page, `History id:` and the mailbox's new history id. `gmail settings filters list`: the id, the criteria, the action. `gmail settings sendas list`: `default`, the verification status, the address, the name. `gmail settings delegates list` and `forwarding-addresses list`: the verification status, the address. A Gmail snippet SHALL print in text with its `<b>` marks stripped and its HTML entities decoded.
- `msgraph attachments list`: the id, `inline`, the size, the type, the name; above, `Message:`. `msgraph mail-folders list|get|child-folders`: the id, the counts, the child folder count, the name; `list` and `child-folders` paged by token.
- `pimdir mailbox list`: the id, the role, the counts, the coverage, the round under way, the name. `pimdir flag list`: one flag per line; above, `Mailbox:` and `Message:`. `pimdir attachment list|search`: the file id, the size, the type, the section, the message id, the mailbox, the date, the sender's name or address, the filename; above, `Query:` for a search; paged by token. `pimdir search --threads`: the thread id, its message count, its last date, the ids of its hits joined with `,`, the senders and subject of its first hit; above, `Query:`; paged by number.

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
- THEN it prints `Account:    work (imap)` and a blank line, then one aligned line per mailbox ending with its name, with no header row and nothing under it

#### Scenario: A page reached by a token
- GIVEN a Gmail account `work` whose drafts fill more than one page
- WHEN `gmail drafts list` prints the first
- THEN it prints `Account:    work (gmail)`, `Page:       ` with the number of drafts it holds, `Next page:  --page-token ` followed by the token, a blank line, then one line per draft

### Requirement: A command given message ids says how many it found
(commands) `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` SHALL require ids, a repeated id being acted on and counted once, and SHALL fail, exit code 1, when none of the given ids exists in the mailbox, changing nothing. When some exist they SHALL succeed and report how many as `count`: the flag commands beside `flags`, the `{raw, iana}` flags written, `raw` spelled as the backend writes it (`$flagged` on JMAP), `message move` and `message copy` beside `queued`, `message delete` beside its `action`. `message delete` in the trash SHALL count the ids the trash holds and remove those alone. On a backend whose ids are account-wide (JMAP, Gmail, Microsoft Graph), an id SHALL count as found only when its message is in the given mailbox, and a move whose source and destination are one mailbox SHALL change nothing. On Maildir and mbox every id SHALL be looked up before the first write, an unknown one skipped, a removal from the trash included. The protocol commands of the local stores taking ids (`maildir` and `mbox` `message copy|move`, `maildir` and `mbox` `flag add|set|remove`) SHALL require them too. The plain output of these commands SHALL end with a newline.

#### Scenario: An id that does not exist
- GIVEN a mailbox holding no message with id `99999999`
- WHEN `himalaya flag add --flag flagged 99999999` runs
- THEN the command exits 1 with `No flag added: no id matched in the mailbox`

#### Scenario: One id of two exists
- GIVEN a mailbox holding message `42` and no message `99999999`
- WHEN `himalaya --json flag add --flag flagged 42 99999999` runs
- THEN the command exits 0 and prints `{"flags":[{"raw":"\\Flagged","iana":"flagged"}],"count":1,"queued":false}`

### Requirement: Unified mailbox writes
(commands) `mailbox create <NAME> [--parent <MAILBOX>]` SHALL create a mailbox named NAME, one level of the hierarchy, at the top of the account or under the parent; `mailbox rename <MAILBOX> <NAME>` SHALL replace the last level of the mailbox's name with NAME, its parent and messages kept; `mailbox delete <MAILBOX>` SHALL delete the mailbox and the messages it holds, the mailbox named with no default. Mailboxes are given by name, alias, role or id. A backend naming the hierarchy by path SHALL join the levels with its own delimiter: the one IMAP's `LIST "" ""` reports, `/` on Gmail, Graph and the local stores. NAME SHALL be refused when empty, blank or holding a control character, and per backend when it holds the hierarchy delimiter (IMAP's, `/` on Gmail, Maildir and the unified mbox writes), or names no listable level (Maildir `cur`, `new`, `tmp`, a leading dot on Maildir). Creating an existing mailbox and renaming onto one SHALL be refused. Deleting a local mailbox that holds others SHALL be refused, naming them; on IMAP a server's refusal of the same SHALL name them too, and a missing mailbox SHALL read ``IMAP mailbox `X` not found`` for rename and delete alike. On IMAP a `--parent` the server marks `\Noinferiors` (RFC 3501 section 7.2.2) SHALL be refused before the `CREATE`, ``IMAP mailbox `X` can hold no other (\Noinferiors)``. On Gmail, where a label nests by name, a rename SHALL also rename every sublabel, as IMAP `RENAME` moves the inferiors; a delete SHALL leave the sublabels under their full names.

The outputs SHALL be `{id, name}` for create and rename, `id` being the mailbox's id afterwards (a rename changes it where the name is the id) and `null` when the write is queued, and `{id}` for delete, the id `mailbox list` prints. `name` SHALL be the full name `mailbox list` prints on IMAP, Maildir and mbox, and NAME as given on the other backends. A write SHALL drop the mailbox index a backend caches, so a later command of the session resolves the new name.

#### Scenario: A Maildir mailbox under a parent
- GIVEN a Maildir account holding `Archive`
- WHEN `mailbox create 2025 --parent Archive`, then `mailbox rename Archive/2025 2026`, then `mailbox delete Archive/2026` run
- THEN `Archive/2025` is created, renamed to `Archive/2026`, then removed

### Requirement: A write prints what it acted on
(commands) Every write SHALL print what it acted on, never a sentence alone. A unified write SHALL print its receipt with `queued`: `message copy` and `message move` `{count, queued}`, `message send`, `message add` (`saved` always true) and a composer's save or send `{id, saved, sent, queued}`, `id` being the saved copy's, `null` when the message was not saved, the save is queued, or the backend returned none. A composer neither saving nor sending SHALL print the template under `--json` and the message bytes otherwise. A protocol write with no payload of its own (Gmail, Microsoft Graph, JMAP, maildir, mbox, ManageSieve) SHALL print `{id}` naming the item it acted on as it is afterwards, or `{ids}` for several, in the order acted on; its text form SHALL keep the sentence. `jmap email import` SHALL print `{id, blobId}`, `id` `null` with `--upload-only`; `msgraph messages send` `{id: null}`, `sendMail` answering without one; `smtp send` the envelope it sent, `{mailFrom, rcptTo}`.

#### Scenario: A Gmail label is created
- GIVEN a Gmail account
- WHEN `himalaya --json gmail labels create Projects` runs
- THEN it prints `{"id":"Label_12"}`, the id Gmail gave the label

### Requirement: Multi-account schema
(config) The config SHALL be multi-account: a top-level block holding shared defaults, plus `[accounts.<name>]` blocks. Each account carries at most one storage backend sub-block (`imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`) and optional service sub-blocks for `smtp` and `sieve`.

The `sieve` block SHALL accept `sieve://`, `sieves://`, and `unix://` servers, and SHALL expose the shared TLS and SASL vocabulary, every mechanism the other backends accept reaching ManageSieve too.

A bare authority SHALL resolve to `sieve://`, where the `imap` and `smtp` blocks resolve theirs to the implicit-TLS scheme: RFC 5804 registers one port, 4190, and defines STARTTLS as the way to TLS on it, so an implicit-TLS default would reach nothing on a stock server. `sieves://` SHALL stay accepted for the deployments listening for a TLS handshake straight away. `sieve.starttls` left unset SHALL follow the scheme, on for `sieve://` and off for the other two, and setting it on a `sieves://` server SHALL be an error.

A mechanism disclosing a reusable credential SHALL be refused over a cleartext connection unless `sieve.allow-cleartext-auth` is set, which is the configuration RFC 5804 section 5 asks for.

### Requirement: Path keys expand as the configuration is read
(config) Every path-valued key SHALL expand `~` and environment variables while the configuration is deserialized, not where it is read. This covers `maildir.root`, `mbox.root`, `mbox.inbox`, `pimdir.root`, the global and per-account `downloads-dir`, and the `tls.cert` of every backend.

An expansion that fails, an undefined variable being the case, SHALL leave the raw value untouched rather than fail the load.

A reader SHALL therefore receive an already-expanded path and SHALL NOT expand again: expansion at a call site holds only where somebody remembered it, which is how `maildir.root = "~/Mail"` came to open a literal `./~/Mail`.

### Requirement: One sample configuration, with a section per backend
(config) The repository SHALL ship config.sample.toml holding one `[accounts.example]` table with dotted keys and no other table header. It SHALL list the options every account shares and each of their values, the default active and the others commented as `#key = value`, every line ending in one tab-aligned comment, and every password command a failing `exit 127` placeholder. It SHALL end with one section per backend or provider (IMAP and SMTP, ManageSieve, JMAP, Gmail, Microsoft Graph, Microsoft IMAP and SMTP, Fastmail, Maildir, mbox, pimdir), each a paragraph of commented code headed by its name, its own alternatives commented twice, an OAuth 2.0 token read through `ortie token show -a example`, so the file holds a working account once one section is uncommented. The test suite SHALL parse config.sample.toml through the loader `-c` uses with each section uncommented whole, then on top of it each line commented twice inside it and each line of commented code outside the sections, in place of the active lines of its field, and SHALL resolve `example` as the default account each time, so a configuration change breaking one fails the build.

#### Scenario: A field is renamed
- GIVEN config.sample.toml, whose IMAP and SMTP section sets `imap.sort.fallback`
- WHEN the field is renamed in the configuration types and the sample is not updated
- THEN the test suite fails naming the sample and the line uncommented

### Requirement: Binary only
(packaging) Himalaya SHALL build as a binary with no public library API and no lib target. A user needing the protocol or storage logic reaches for the io-* library that owns it (io-imap, io-jmap, io-gmail, io-msgraph, io-smtp, io-managesieve, io-maildir, io-mbox, io-pimdir).

### Requirement: Feature-gated backends
(packaging) Every backend SHALL sit behind its own cargo feature (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`), so a build ships only the protocols it needs. A protocol command and its backend adapter compile only when its feature is on.

### Requirement: Per-backend translation
(search) Each backend SHALL translate the shared query into its native surface. IMAP translates to `UID SEARCH` / `UID SORT` keys (`SENTBEFORE` for `before`, an `OR` of `FROM`, `TO`, `CC` and `BCC` for `address`), negations pushed down to single keys (double negations cancelled, De Morgan over `and` and `or`, a negated `address` sent as `NOT FROM x NOT TO x NOT CC x NOT BCC x`), a root `and` sent as the list of its keys, a `flag` naming an IANA keyword sent in its canonical spelling (`flag junk` as `KEYWORD $Junk`), and a UID the server's `SORT` lists twice kept once, with a client-side sort fallback when the server lacks `SORT` or is iCloud (provider-quirks), which orders as RFC 5256 defines (base subject, first address's mailbox, ties in sequence order). JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id; on the chain of `and` from the root a date clause over-approximates on the received date and is re-checked exactly on the sent date client-side, while under `or` or `not` it reads the received date alone, a client-side residual being unable to hold across a disjunction or a negation. Maildir and mbox list then evaluate a shared client-side matcher (`matches_filter` plus `sort_envelopes`) over the fetched envelopes, reusing the read bytes for body clauses and reading the `Cc:` and `Bcc:` headers off them only when a clause needs them. `sort_envelopes` sorts as RFC 5256 `SORT` does: `subject` on the base subject (section 2.1), `from` and `to` on the mailbox of the first address, both under `i;ascii-casemap`, ties keeping their order. pimdir evaluates the same matcher over envelopes built from the stored summaries, the copy recipients and the attachment mark coming from the summary, reading a message's local body only when the filter holds a body clause; a message whose body is not local matches no body clause. Gmail translates to a `q` value, each pattern a literal: quotes, braces and parentheses become spaces, a standalone `OR` or `AND` is lowercased, anything but a bare word is quoted, and a pattern with no word left is refused with `search-unsupported`; its sort uses the shared `sort_envelopes`; a date clause as Unix seconds bounding the UTC day (`after d` from the start of the next day, one second early so a boundary message is kept, `before d` to the start of that day), Gmail reading a bare date as midnight in PST and comparing it with the message's received date. Graph translates to a KQL `$search` (`participants` for `address`, `hasAttachments` for `attachment`), each word of a pattern a quoted KQL phrase, so a word that is KQL syntax (`OR`, `x:y`, `alpha*`) is matched as a word, the words of one pattern grouped in parentheses and a double quote dropped, and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it.

#### Scenario: A pimdir body clause
- GIVEN a pimdir mailbox holding two local bodies, one containing `attached`, and one message whose body is not local
- WHEN `envelope list -m imap/INBOX body attached` runs
- THEN only the message whose local body holds `attached` is listed, and `not body attached` lists the two others

#### Scenario: A negated date clause on JMAP
- GIVEN a JMAP mailbox holding a message sent on 2025-12-01
- WHEN `envelope search not after 2026-01-01` runs
- THEN the message is listed

## REMOVED Requirements

None: m2dir appeared inside shared requirements only. The m2dir test reports (cairn/spec/testing/m2dir.md, m2dir-specific.md) are removed.
