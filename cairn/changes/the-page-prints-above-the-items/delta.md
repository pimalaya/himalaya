---
cairn: delta
change: the-page-prints-above-the-items
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Text output is plain text
(commands) A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

Every list SHALL print through pimalaya-cli's `Columns` with no header row, one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-` and a line break inside a cell as a space. Its short cells SHALL read first and its free text (a subject, a name, a path, an error) last; a cell holding several values SHALL join them with `,`, a size SHALL print with no blank (`1KiB`), a mailbox's unread and total counts SHALL print as one cell, unread first (`3/120`), a count the backend gave none of as `-`, and a yes-or-no cell SHALL print its word (`default`, `active`, `permanent`, `inline`, `subscribed`) or nothing, so a line splits on blanks.

Above the items SHALL print `Account:` (the account name, then in parentheses the backend answering the whole list) and the fields naming what is listed (`Mailbox:`, `Message:`, `Query:` and the others below). After them, a paged list SHALL print `Page:` and, while a page follows, `Next page:` with the flag reaching it: a list paged by number names the page and the items it holds (`2, envelopes 26 to 50`, followed by `, the last` when no page follows, or `empty`; `all` for an unpaged listing) and gives `--page`; a list paged by token counts them (`25 messages`, `, the last`, `empty`) and gives `--page-token`. The other fields placing the answer (`Incomplete:`, `History id:`, `Cancel:`) SHALL follow in the same block, and a blank line SHALL part the block from the items, nothing printing under them. Field values SHALL start in one column on every list, two spaces after `Next page:`. A list reading no account (`account list`, `maildir flag list`, `mbox flag list`) SHALL print its items alone; a list without items, its header alone.

A list of messages SHALL print each the way `mscan` prints a mailbox: its id; four flag columns, `*` for no `\Seen`, `R` for answered, `!` for flagged and `@` only when the backend says the message has an attachment, each a space otherwise; its date as `envelope.list.datetime-fmt` (`%F %R` by default) and `envelope.list.datetime-local-tz` (true by default) configure it; the list's own short cells; its senders' display names, the address standing for a nameless one, joined with `, `; then its subject, last. These lists print so:

- `envelope list|search`: the envelopes in the listing's order; the senders, or `to ` and the recipients with `--recipient`. The size SHALL NOT print. Above: `Mailbox:` (its alias resolved) and, for a search given a query, `Query:` (the query as parsed). Paged by number.
- `jmap email query|get`: the date is `receivedAt`. Above: `Mailbox:` when `--mailbox` filters. A query is paged by number, a page following while one comes back full.
- `msgraph messages list`: above, `Folder:`, `Search:` and `Filter:` when given. Paged by token.
- `pimdir search`: its mailboxes, joined with `,`, then, when a text term matched any hit, `body` or `headers`; the snippet SHALL NOT print. Above: `Query:`. Paged by number; after the page, `Incomplete:` and what the answer could not see, when it is not whole.
- `imap thread`: each thread from its first message, the subject indented two spaces per level: a run of members nests each under the one before, the answers sit under the last, and the answers of a message the mailbox lacks stand side by side at its level. A message whose envelope could not be fetched prints its id, its other cells empty. Above: `Mailbox:` unless `--no-select`.
- `pimdir thread`: each message after the earlier one its `In-Reply-To` names, its subject indented one level further, one whose parent the account lacks standing at the margin; its mailboxes, joined with `,`, before the senders. Above: `Thread:` when a thread holds the message.

`pimdir tag` and `pimdir queue list` SHALL print the same shape with what they carry. `pimdir tag`: the message id; the flags once the tag applies, blank where they were never read; the status (`planned`, `queued`, `unchanged`, `flags-unknown`, `several-sources`, `not-synced`, `unsupported`); the mailbox; the queue row; the subject. Above: `Query:` and `Incomplete:` as for a search. `pimdir queue list`: the row; `queued` or `parked`; the staged flags, blank for a parked row; when it was queued; the action (`save` or `send`, a parked row's kind); `to ` and the recipients' names; then the subject, or why the row was parked. Above: `Mailbox:` and `Cancel:` with the command cancelling a row, when any is listed.

The other lists SHALL print these cells, in this order:

- `mailbox list`: the id, the role, with `--counts` the counts, the name.
- `attachment list|download`: the part id, `inline`, the size, the type, then the filename, or the path once written. Above: `Mailbox:` and `Message:`.
- `account list`: the name, the backends, `default`. `account capability`: the capability, the backend, the level, the detail; `Account:` names no backend.
- `sieve list`: `active`, the name.
- `imap list`: the delimiter, the role, the attributes, the name. `imap flag`: the flag, `permanent`; above, `Mailbox:`. `imap search|sort`: one id per line; above, `Mailbox:` unless `--no-select`.
- `maildir list`, `m2dir list`, `mbox list`: the name, the path. `m2dir flag list`: one flag per line; above, `Mailbox:` and `Message:`. `maildir flag list`: the letter, the name; `mbox flag list`: the letter, the header, the name.
- `jmap mailbox query|get`: the id, the role, the counts, `subscribed`, the name; a query paged by number. `jmap submission query|get|create`: the id, the email id, the identity id, the undo status, the send date; a query paged by number. `jmap identity get`: the id, the email, the name. `jmap thread get`: one email per line, beside its thread.
- `gmail messages list`: the id, the thread id. `gmail threads list`: the id, the history id, the snippet. Both above: `Labels:` and `Query:` when given. `gmail drafts list`: the draft id, the message id, the thread id; above, `Query:`. The three paged by token. `gmail labels list|get`: the id, the type, the counts, the name. `gmail history list`: one line per message change, the record id, `added`, `deleted`, `labeled` or `unlabeled`, the message id, then the labels; above, `Since:` and `Label:`; paged by token, counting records; after the page, `History id:` and the mailbox's new history id. `gmail settings filters list`: the id, the criteria, the action. `gmail settings sendas list`: `default`, the verification status, the address, the name. `gmail settings delegates list` and `forwarding-addresses list`: the verification status, the address.
- `msgraph attachments list`: the id, `inline`, the size, the type, the name; above, `Message:`. `msgraph mail-folders list|get|child-folders`: the id, the counts, the child folder count, the name.
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

## REMOVED Requirements

None.
