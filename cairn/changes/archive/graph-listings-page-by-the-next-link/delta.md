---
cairn: delta
change: graph-listings-page-by-the-next-link
---

# Delta

## ADDED Requirements

## MODIFIED Requirements

### Requirement: Text output is plain text
A text output SHALL be plain text: no border, no colour, nothing sized to or truncated for the terminal, and it SHALL end with a newline, the explicitly requested raw bytes (`message read --raw`, `attachment download --stdout`, `message parse --part` and `--section`) staying byte-exact.

Every list SHALL print through pimalaya-cli's `Columns` with no header row, one item per line, the cells two spaces apart and padded to the widest of their column, an empty cell as `-` and a line break inside a cell as a space. Its short cells SHALL read first and its free text (a subject, a name, a path, an error) last; a cell holding several values SHALL join them with `,`, a size SHALL print with no blank (`1KiB`), a mailbox's unread and total counts SHALL print as one cell, unread first (`3/120`), a count the backend gave none of as `-`, and a yes-or-no cell SHALL print its word (`default`, `active`, `permanent`, `inline`, `subscribed`) or nothing, so a line splits on blanks.

Above the items SHALL print `Account:` (the account name, then in parentheses the backend answering the whole list) and the fields naming what is listed (`Mailbox:`, `Message:`, `Query:` and the others below). After them, a paged list SHALL print `Page:` and, while a page follows, `Next page:` with the flag reaching it: a list paged by number names the page and the items it holds (`2, envelopes 26 to 50`, followed by `, the last` when no page follows, or `empty`; `all` for an unpaged listing) and gives `--page`; a list paged by token counts them (`25 messages`, `, the last`, `empty`) and gives `--page-token`. The other fields placing the answer (`Incomplete:`, `History id:`, `Cancel:`) SHALL follow in the same block, and a blank line SHALL part the block from the items, nothing printing under them but the errors of the items the command could not serve ("An item that failed prints last"). Every list SHALL print through pimalaya-cli's `listing::Listing`, its field values starting in one column, two spaces past the colon of the longest label and never before the column `Next page:` sets, so they align on every page; `Incomplete:` and `History id:` widen it. A list reading no account (`account list`, `maildir flag list`, `mbox flag list`) SHALL print its items alone; a list without items, its header alone, unless it says otherwise below.

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
- `gmail messages list`: the id, the thread id. `gmail threads list`: the id, the history id, the snippet. Both above: `Labels:` and `Query:` when given. `gmail drafts list`: the draft id, the message id, the thread id; above, `Query:`. The three paged by token. `gmail labels list|get`: the id, the type, the counts, the name. `gmail history list`: one line per message change, the record id, `added`, `deleted`, `labeled` or `unlabeled`, the message id, then the labels; above, `Since:` and `Label:`; paged by token, counting records; after the page, `History id:` and the mailbox's new history id. `gmail settings filters list`: the id, the criteria, the action. `gmail settings sendas list`: `default`, the verification status, the address, the name. `gmail settings delegates list` and `forwarding-addresses list`: the verification status, the address. A Gmail snippet SHALL print in text with its `<b>` marks stripped and its HTML entities decoded.
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
- THEN it prints `Account:    work (imap)` and a blank line, then one aligned line per mailbox ending with its name, with no header row and nothing under it

#### Scenario: A page reached by a token
- GIVEN a Gmail account `work` whose drafts fill more than one page
- WHEN `gmail drafts list` prints the first
- THEN it prints `Account:    work (gmail)`, `Page:       ` with the number of drafts it holds, `Next page:  --page-token ` followed by the token, a blank line, then one line per draft

### Requirement: JSON keys are camelCase
Every output type handed to the printer SHALL serialize its keys as camelCase, matching the wire formats the backends speak (JMAP per RFC 8620, Microsoft Graph, the Google APIs) and keeping every key reachable by dot access in jq and JavaScript. Each one SHALL be named `<Domain><Target><Verb>Output`, the domain omitted for the unified commands. A field carrying a provider spelling verbatim SHALL keep its explicit `#[serde(rename)]`, the provider's names not being derivable ones (a provider's paging cursor, `@odata.nextLink` or `nextPageToken`, being the one key lifted, into `nextPage`: "Listings resume from their cursor"), and a transparent newtype over an io-gmail or io-msgraph resource SHALL serialize that resource as its crate declares it. The rule covers keys only: string values keep their own spelling (error codes, flags, mailbox roles, `message delete` actions and pimdir queue states stay kebab-case or lowercase). Configuration types SHALL stay kebab-case: they are read from TOML and are not `--json` payloads.

### Requirement: Listings resume from their cursor
`envelope list` and `envelope search` SHALL print `nextPage` beside `envelopes`: the next page number, as a string, while a page follows the one printed, `null` once the page reaches the end of the listing, an exactly full last page included, and SHALL take it back through `--page-token`, which wins over `--page`. Whether a page follows SHALL come from the listing's own exchanges: one does when the page ends before the count the backend holds (the messages Maildir and mbox list or match, the `EXISTS` of IMAP's `SELECT` for a listing and the UIDs its `SORT` answers for a search, the `total` of JMAP's `Email/query`, the items pimdir counts for a listing), when pimdir's search finds one hit past the page, when the listing of Gmail or Graph holds one message past the page, which the page asks along (`maxResults` or `$top` one longer, a later Gmail page and a Graph search reading the pages before it in as few calls as the largest page allows, the next page token or link followed only while the listing runs short of that message), both answering a next page token or link after an exactly full last page too, and, on a JMAP server computing no `total`, when the page comes back full. Pages SHALL be numbered from 1: `--page 0` and a page token `0` are refused, `--page-size 0` listing the whole mailbox. `msgraph messages list`, `msgraph mail-folders list` and `msgraph mail-folders child-folders` SHALL print Graph's next link (`@odata.nextLink`) as `nextPage`, as Graph gives it, a `$search` page's included, and not beside it, `null` without one or when it leads back to the page asked (`--top 0`); `--page-token` SHALL follow it as is, the query it carries replacing the other options, a link off the origin of Graph's API (`https://graph.microsoft.com`) being refused before any request, the bearer token never leaving it. `msgraph attachments list` SHALL follow every next link, so it never stops early and prints no cursor. `msgraph messages list --search` SHALL quote its value for Graph, its double quotes escaped, unless it opens on a double quote, and `--count` SHALL print Graph's `@odata.count` as `count` (`Count:` in text), `null` without it or beside `--search`, Graph ignoring `$count` there. The Gmail listings (`gmail messages list`, `threads list`, `drafts list`, `history list`) SHALL print Gmail's `nextPageToken` as `nextPage`, and not beside it, and take it back through `--page-token`; Gmail's other lists hold no cursor. `jmap email query`, `jmap mailbox query` and `jmap submission query` SHALL print `nextPage` beside their collection the same way, a page following while the query's `total` lies past it, or, on a server computing none, while it comes back full, and SHALL take it back through `--page-token`, a token that is no page number from 1 being refused; `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create`, printing the same shape, SHALL print `nextPage` as `null`.

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

## REMOVED Requirements
