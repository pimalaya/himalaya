---
cairn: delta
change: the-list-frame-is-shared
---

# Delta

## ADDED Requirements

### Requirement: An item that failed prints last
(commands) In the text output, the `errors` entries SHALL print last, as pimalaya-cli's `error::ItemErrors` prints them: after a list's items or a write's sentence, one blank line, then one `Error:` line per entry, in order, its cells the id, `-` when null, the code, `-` when null, then the message, aligned among the entries as columns are. Nothing SHALL print, blank line included, when there is no entry.

#### Scenario: One unknown id of a write
- GIVEN a mailbox holding message `42` and no message `99`
- WHEN `himalaya flag add --flag flagged 42 99` runs
- THEN it prints `Successfully added flags to 1 message(s): \Flagged`, a blank line, then `Error:  99  message-not-found  …`, and the exit status is 3

#### Scenario: An unreadable envelope
- GIVEN an IMAP page holding envelope `4` and a FETCH line naming no UID that does not decode
- WHEN `himalaya envelope list` prints it
- THEN it prints the fields, a blank line, the line of `4`, a blank line, then `Error:  -  message-unreadable  …`

## MODIFIED Requirements

### Requirement: Text output is plain text
(commands) A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

Every list SHALL print through pimalaya-cli's `Columns` with no header row, one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-` and a line break inside a cell as a space. Its short cells SHALL read first and its free text (a subject, a name, a path, an error) last; a cell holding several values SHALL join them with `,`, a size SHALL print with no blank (`1KiB`), a mailbox's unread and total counts SHALL print as one cell, unread first (`3/120`), a count the backend gave none of as `-`, and a yes-or-no cell SHALL print its word (`default`, `active`, `permanent`, `inline`, `subscribed`) or nothing, so a line splits on blanks.

Above the items SHALL print `Account:` (the account name, then in parentheses the backend answering the whole list) and the fields naming what is listed (`Mailbox:`, `Message:`, `Query:` and the others below). After them, a paged list SHALL print `Page:` and, while a page follows, `Next page:` with the flag reaching it: a list paged by number names the page and the items it holds (`2, envelopes 26 to 50`, followed by `, the last` when no page follows, or `empty`; `all` for an unpaged listing) and gives `--page`; a list paged by token counts them (`25 messages`, `, the last`, `empty`) and gives `--page-token`. The other fields placing the answer (`Incomplete:`, `History id:`, `Cancel:`) SHALL follow in the same block, and a blank line SHALL part the block from the items, nothing printing under them but the errors of the items the command could not serve ("An item that failed prints last"). Every list SHALL print through pimalaya-cli's `listing::Listing`, its field values starting in one column, two spaces past the colon of the longest label and never before the column `Next page:` sets, so they align on every page; `Incomplete:` and `History id:` widen it. A list reading no account (`account list`, `maildir flag list`, `mbox flag list`) SHALL print its items alone; a list without items, its header alone, unless it says otherwise below.

A list of messages SHALL print each the way `mscan` prints a mailbox: its id; four flag columns, `*` for no `\Seen`, `R` for answered, `!` for flagged and `@` only when the backend says the message has an attachment, each a space otherwise; its date as `envelope.list.datetime-fmt` (`%F %R` by default) and `envelope.list.datetime-local-tz` (true by default) configure it; the list's own short cells; its senders' display names, the address standing for a nameless one, joined with `, `; then its subject, last. These lists print so:

- `envelope list|search`: the envelopes in the listing's order; the senders, or `to ` and the recipients with `--recipient`. The size SHALL NOT print. Above: `Mailbox:` (its alias resolved) and, for a search given a query, `Query:` (the query as parsed). Paged by number.
- `jmap email query|get`: the date is `receivedAt`. Above: `Mailbox:` when `--mailbox` filters. A query is paged by number, a page following while one comes back full.
- `msgraph messages list`: the date is `sentDateTime`, else `receivedDateTime`. Above, `Folder:`, `Search:` and `Filter:` when given, and `Count:` with `--count`. Paged by token, a next link that is no token printed as `Next link:`.
- `pimdir search`: its mailboxes, joined with `,`, then, when a text term matched any hit, `body` or `headers`; the snippet SHALL NOT print. Above: `Query:`. Paged by number; after the page, `Incomplete:` and what the answer could not see, when it is not whole.
- `imap thread`: each thread from its first message, the subject indented two spaces per level: a run of members nests each under the one before, the answers sit under the last, and the answers of a message the mailbox lacks stand side by side at its level. A message whose envelope could not be fetched prints its id, its other cells empty. Above: `Mailbox:` unless `--no-select`.
- `pimdir thread`: each message after the earlier one its `In-Reply-To` names, its subject indented one level further, one whose parent the account lacks standing at the margin; its mailboxes, joined with `,`, before the senders. Above: `Thread:` when a thread holds the message.

`pimdir tag` and `pimdir queue list` SHALL print the same shape with what they carry. `pimdir tag`: the message id; the flags once the tag applies, blank where they were never read; the status (`planned`, `queued`, `unchanged`, `flags-unknown`, `several-sources`, `not-synced`, `unsupported`); the mailbox; the queue row; the subject. Above: `Query:` and `Incomplete:` as for a search. `pimdir queue list`: the row; `queued`, `parked` or `unconfirmed`; the staged flags, blank for a parked row; when it was queued; the action (`save` or `send`, a parked row's kind); `to ` and the recipients' names; then the subject, or why the row was parked. Above: `Mailbox:` and `Cancel:` with the command cancelling a row, when any is listed.

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

### Requirement: An account lists what it serves
(commands) `account capability` SHALL print pimalaya-cli's `capability::AccountCapabilityOutput`, one row per unified capability, `{name, backend, level, detail}` under `{account, capabilities}`: the backend the resolver picks for the account and `--backend`, `null` when none is configured, and that backend's level and detail. It SHALL read the configuration and the capability tables and connect nowhere. On a pimdir account, a write SHALL be read from what the store's sources declare: an item write through the store's gate in each of the account's mail collections (`none` refused in all, `partial` refused in some or passing in part), an intent served when a source performs it, the static row kept when no source declares anything.

#### Scenario: A Maildir synced from JMAP
- GIVEN an account configuring a Maildir and a JMAP block
- WHEN `account capability` runs
- THEN `mailbox.list` names `maildir`, `vacation` names `jmap` at `server`, and nothing connects

### Requirement: The server tells the agent how to use it
(mcp) `initialize` SHALL carry `instructions` saying how to find a mailbox (`mailbox_list`, the `mailbox` argument, the inbox when left out) and an envelope id (`envelope_list`, `envelope_search` and its query language), that `attachment_download` returns the bytes as an embedded resource and the composers take files inline as `attach-data`, that `--allow-write` and `--allow-send` gate the tools and arguments that change the store and send mail, that `account_capability` says which backend serves each capability of the account and how well, and that a failed call answers `isError` with the `himalaya-error` object. Every tool name it holds SHALL be one of the tool table.

### Requirement: Tools are the classified unified API
(mcp) The server SHALL expose one tool per leaf command of the unified API that is classified as reading, writing or sending and that the account's transports serve at least one capability of, as the static tier knows without connecting, named `<group>_<leaf>`, and no protocol-specific or raw passthrough command. It SHALL also expose `account capability` as `account_capability`, listed whatever the account serves and whatever the flags, annotated `readOnlyHint` true, `destructiveHint` false and `openWorldHint` false. A tool whose capability only the server decides SHALL stay listed and answer `capability-missing` as a tool error when the server lacks it. An argument reaching a capability of its own (the composers' `send` and `no-save` reach `submit`, their `save` reaches `message.add`) SHALL be left out of the tool where the static tier says the account does not serve it, the tool staying listed.

#### Scenario: A send-only account
- WHEN `himalaya mcp --allow-write --allow-send` serves an account configuring SMTP alone
- THEN `tools/list` holds `message_send` and `message_compose`, and no `envelope_list` or `flag_add`

#### Scenario: What a Maildir account serves
- WHEN `himalaya mcp` serves an account configuring a Maildir and SMTP, with no flag
- THEN `tools/list` holds `account_capability`, read-only, and calling it answers the rows `account capability` prints, `mailbox.list` on `maildir` and `submit` on `smtp`

## REMOVED Requirements

### Requirement: An item that failed prints in the header block
(commands) Replaced by "An item that failed prints last": the owner decided on 2026-11-14 that item errors print after the output, as pimalaya-cli prints them.
