---
cairn: spec
capability: search
status: current
---

# Search

Unified envelope search is expressed in one backend-agnostic query language, himalaya's own, parsed once and translated per backend. The query type carries a filter tree and a sort list; the parser is built on chumsky and renders its errors through the CLI's ariadne setup. Each backend matches text its own way, and refuses a clause it cannot search rather than dropping it.

A pimdir account also answers the pimdir SEARCH query language natively, from the store's search index, under `himalaya pimdir`; the shared language stays himalaya's.

### Requirement: One query language
The `envelope search` query SHALL parse into a single `SearchEmailsQuery` (filter plus sort), independent of the backend. The grammar covers the `date`, `after` and `before` date clauses, taking the date values of "Date values can be relative or instants", `after` and `before` both strict, the `from`, `to`, `cc` and `bcc` address clauses and `address` over all four, the `subject` and `body` text clauses, the `flag` clause over any flag name, the valueless `attachment` clause and the `message-id` clause, taking a whole id with or without its angle brackets, combined with `not`, `and` and `or`, binding in that order, grouped with parentheses, and an `order by` list over date, from, to and subject. The sort comes last, introduced by the words `order by` outside any pattern, and lists at least one key. Every keyword of the grammar is a whole word. A pattern opening with a double quote is a quoted pattern, refused when unterminated; an unquoted pattern takes `\"`, `\\`, `\ `, `\(` and `\)` escapes. Blanks after `(` and before `)` are insignificant.

### Requirement: Date values can be relative or instants
A date clause SHALL take a day (`2026-11-14`, `2026/11/14`, `14-11-2026`, `14/11/2026`, its year written with four digits and its month and day with one or two, `today`, `yesterday`, `<n> day|week|month|year[s] ago`, counted back in calendar terms) or an instant (`<n> minute|hour[s] ago`, an RFC 3339 date-time whose seconds and offset are optional, its fraction of a second dropped). A year of another length SHALL be refused with `search-invalid`, so `15-11-26` is no date. Days and offset-less instants SHALL be read in the local zone, `TZ` then the system's zone, falling back to UTC with one warning when neither names one; now SHALL be taken once per query. `after` and `before` SHALL compare an instant to the second, strictly; `date` SHALL refuse an instant with `search-invalid`. A relative value SHALL be resolved at parse time. The local evaluator (Maildir, mbox, pimdir `envelope search`) SHALL compare an instant to the envelope's date; Gmail SHALL send it as Unix seconds in `after:` and `before:`; JMAP SHALL bound the received date with it in UTC and, on the chain of `and` from the root, re-check the sent date. IMAP SHALL serve an instant on the chain of `and` from the root exactly: it searches the days from the one before the instant's UTC day (`after`, `SENTSINCE`) or up to the second after it (`before`, `SENTBEFORE`), every message of that three-day band, whose `Date:` may fall on either side of the instant whatever its offset, searched with the rest of the query, its date fetched and kept only on the instant's side, before the page is cut; under `or` or `not` it SHALL refuse it with `search-unsupported`. Microsoft Graph, whose KQL `sent` compares days, SHALL refuse an instant with `search-unsupported` until a time of day is shown to work there.

#### Scenario: Two days ago is a day
- GIVEN the local date is 2026-11-14
- WHEN `himalaya envelope search after 2 days ago` runs
- THEN it searches as `after 2026-11-12` does

#### Scenario: Five hours ago is an instant
- GIVEN it is 2026-11-14T15:00 in the local zone, and an IMAP mailbox holding a message sent at 09:00 and one at 11:00 that day
- WHEN `himalaya envelope search after 5 hours ago` runs
- THEN only the message sent at 11:00 is listed

#### Scenario: An instant under or on IMAP
- WHEN `himalaya envelope search after 5 hours ago or from alice` runs on an IMAP account
- THEN it fails with `search-unsupported`, nothing searched

#### Scenario: A two-digit year
- WHEN `himalaya envelope search after 15-11-26` runs
- THEN it fails with `search-invalid`

### Requirement: The query's clauses are listed on -h
The first paragraph of the `QUERY` argument's help, which `-h` shows, SHALL name every clause, operator and the sort, comma separated, and point at `--help`, whose remaining paragraphs give each clause's syntax, examples and the precedence. The command's long help SHALL say how each backend matches text. Options MAY follow the query words; a word opening on a hyphen goes after `--`.

### Requirement: A query that does not parse is refused with a code
A query the grammar refuses SHALL fail with the code `search-invalid`, the message ``Cannot parse search query `<query>` `` followed by the parser's diagnostics drawn with ariadne over the whole query as typed, each naming the word found and what was expected there. A query nesting parentheses deeper than 32 levels, or holding more than 256 `and`, `or` and `not` (a bare one counting wherever it stands outside quotes), SHALL be refused the same way before the grammar reads it, the diagnostic naming the bound and pointing at the first parenthesis or operator past it.

### Requirement: A clause a backend does not answer is refused with a code
A query holding a clause the account's backend cannot compile SHALL be refused with the code `search-unsupported`; nothing is searched and no clause is dropped. A clause or sort key the backend declares `none` SHALL be refused before connecting, naming the option and the backend; one it answers in part SHALL be refused while it is translated, before any request, the message quoting the clause as the language writes it and saying why. IMAP refuses `attachment`, Gmail `flag answered` and keyword flags, Microsoft Graph every flag clause and a `message-id` clause beside any other, JMAP `message-id`.

### Requirement: What a backend searches is declared
Each backend SHALL declare, per clause and per sort key, an `envelope.search.<option>` capability (`envelope.search.from`, `envelope.search.sort.date`), and its `envelope.search` level SHALL say how it matches text: the local evaluator by case-insensitive substring, `subject` and `body` alike lowercasing Unicode text (`full`); IMAP as the server matches it, RFC 3501 asking for a case-insensitive substring and some servers (Cyrus, Fastmail) matching whole words, JMAP as the server defines `text` and the header conditions (RFC 8621 section 4.4.1), Gmail by words as its search box does, Graph by KQL words in relevance order (`partial`). Gmail's and JMAP's date clauses are `partial`, comparing the received date; Gmail's `body` matches headers too, and its `attachment` counts inline images (`partial`); sorting on Gmail and Graph orders the fetched page only; pimdir's `body` misses a body not local yet.

### Requirement: Per-backend translation
Each backend SHALL translate the shared query into its native surface. IMAP translates to `UID SEARCH` / `UID SORT` keys (`SENTBEFORE` for `before`, an `OR` of `FROM`, `TO`, `CC` and `BCC` for `address`), negations pushed down to single keys (double negations cancelled, De Morgan over `and` and `or`, a negated `address` sent as `NOT FROM x NOT TO x NOT CC x NOT BCC x`), a root `and` sent as the list of its keys, a `flag` naming an IANA keyword sent in its canonical spelling (`flag junk` as `KEYWORD $Junk`), and a UID the server's `SORT` lists twice kept once, with a client-side sort fallback when the server lacks `SORT` or is iCloud (provider-quirks), which orders as RFC 5256 defines (base subject, first address's mailbox, ties in sequence order). JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id; on the chain of `and` from the root a date clause over-approximates on the received date, a day bounded from below by its first instant in any offset (10:00 UTC the day before, at UTC+14:00) so a message dated in an offset east of UTC is found on its own day, and is re-checked exactly on the sent date client-side, while under `or` or `not` it reads the received date alone, a client-side residual being unable to hold across a disjunction or a negation. Maildir and mbox list then evaluate a shared client-side matcher (`matches_filter` plus `sort_envelopes`) over the fetched envelopes, reusing the read bytes for body clauses and reading the `Cc:` and `Bcc:` headers off them only when a clause needs them. `sort_envelopes` sorts as RFC 5256 `SORT` does: `subject` on the base subject (section 2.1), `from` and `to` on the mailbox of the first address, both under `i;ascii-casemap`, ties keeping their order. pimdir evaluates the same matcher over envelopes built from the stored summaries, the copy recipients and the attachment mark coming from the summary, reading a message's local body only when the filter holds a body clause; a message whose body is not local matches no body clause. Gmail translates to a `q` value, a conjunction as its terms joined by blanks and a disjunction as a brace group, an alternative holding more than one term (a conjunction, the two bounds of a `date` clause, a negated `address` or disjunction) wrapped in parentheses, Gmail reading every blank-separated term inside braces as an alternative of its own; each pattern is a literal: quotes, braces and parentheses become spaces, a standalone `OR` or `AND` is lowercased, anything but a bare word is quoted, and a pattern with no word left is refused with `search-unsupported`; its sort uses the shared `sort_envelopes`; a date clause as Unix seconds bounding the UTC day (`after d` from the start of the next day, one second early so a boundary message is kept, `before d` to the start of that day), Gmail reading a bare date as midnight in PST and comparing it with the message's received date. Graph translates to a KQL `$search` (`participants` for `address`, `hasAttachments` for `attachment`), each word of a pattern a quoted KQL phrase, so a word that is KQL syntax (`OR`, `x:y`, `alpha*`) is matched as a word, the words of one pattern grouped in parentheses and a double quote dropped, and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it. A `message-id` clause SHALL match the whole id, compared byte for byte with the envelope's bare `Message-ID:`: IMAP sends `HEADER Message-ID`, which the server matches as a substring, and keeps on the chain of `and` from the root only the fetched envelopes carrying the id, a substring match remaining under `or` and `not` (`partial`); Gmail sends `rfc822msgid:`; Graph answers a lone `message-id` clause with a `$filter` on `internetMessageId`, `$filter` taking no `$search` beside it (`partial`); the local evaluator compares the envelope's id; JMAP declares it `none`, io-jmap carrying no `header` condition.

#### Scenario: A pimdir body clause
- GIVEN a pimdir mailbox holding two local bodies, one containing `attached`, and one message whose body is not local
- WHEN `envelope list -m imap/INBOX body attached` runs
- THEN only the message whose local body holds `attached` is listed, and `not body attached` lists the two others

#### Scenario: A negated date clause on JMAP
- GIVEN a JMAP mailbox holding a message sent on 2025-12-01
- WHEN `envelope search not after 2026-01-01` runs
- THEN the message is listed

### Requirement: Graph flag clauses
Graph SHALL refuse a filter holding a flag clause with the code `search-unsupported`, naming the flag: KQL has no flag property and Graph accepts no `$filter` beside `$search`, so dropping the clause would widen the result.

### Requirement: pimdir searches through the store's index
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-draft-13 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. The answer SHALL be the store as of the owner's last refresh: a queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort date|name|relevance` orders the hits (date by default), `--page-size` and `--page-token` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date as `firstDate` and `lastDate`, and its hits. Under `--json` the page SHALL print `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`: `nextPage` the token of the next page while hits or threads follow the page, `null` once the answer is complete, which `--page-token` SHALL take back: the place of the last hit or thread the page answered (io-pimdir `PimdirSearchCursor`), so the next page starts after it whatever the index gained or lost since, bound to the account, the query, the order and `--threads` it was printed under. A token another account, query, order or listing printed, or one no search printed, SHALL be refused, coded `page-token-invalid`. A page SHALL be reached by its token alone, `pimdir search` taking no page number. A hit whose message the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other hits listed (a thread without that hit) and the exit status 3; in text the entries print after the items, as every list prints them.

A `date:` term's years, months and days, `today` and `yesterday` SHALL be read in the local zone (SEARCH §8.1), in `pimdir search`, `count` and `tag` alike: `TZ`, then the system's zone, by its IANA name. A zone with no IANA name, a POSIX `TZ` string or a system zone that cannot be named, or no zone at all, SHALL leave them in UTC with one warning.

#### Scenario: A body match
- GIVEN a pimdir store whose index holds a message whose body says `quarterly`
- WHEN `himalaya pimdir search quarterly` runs
- THEN the message is listed with `matched` `body` and a snippet holding `quarterly`

#### Scenario: A staged message is not listed
- GIVEN a message added to a pimdir mailbox and still in the queue
- WHEN `himalaya pimdir search` runs a query its body matches
- THEN it is not listed until the owner applies the add and refreshes the index

#### Scenario: A second page of a search
- GIVEN a pimdir index holding three messages `report` matches
- WHEN `himalaya --json pimdir search -s 2 report` runs, then `himalaya --json pimdir search -s 2 --page-token <nextPage> report` with the `nextPage` it printed
- THEN the first prints two hits and a `nextPage` token, the second the third hit and `"nextPage":null`

#### Scenario: A token of another query
- GIVEN the `nextPage` of `himalaya --json pimdir search -s 2 report`
- WHEN `himalaya --json pimdir search --page-token <nextPage> --sort name report` runs
- THEN it fails coded `page-token-invalid`, exit 1

#### Scenario: A hit the store cannot read
- GIVEN a pimdir index holding three messages `report` matches, the store failing to read the row of one
- WHEN `himalaya --json pimdir search report` runs
- THEN it prints the two other hits and `"errors":[{"id":"<its id>","code":"message-unreadable",…}]`, and the exit status is 3

#### Scenario: A day is the local day
- GIVEN `TZ=Europe/Paris` and an indexed message received at 00:30 on 2026-11-07 in that zone
- WHEN `himalaya pimdir search date:2026-11-07` runs
- THEN the message is listed, and `date:2026-11-06` does not list it

### Requirement: pimdir counts, threads and tags from the index
`himalaya pimdir count <query>...` SHALL answer how many messages the scoped query matches, with its coverage. `himalaya pimdir thread <id>` SHALL list the thread of one message, oldest first, a message no thread holds being its own. `himalaya pimdir tag <query>... --add <flag> --remove <flag>` SHALL stage one `set-flags` per placement each hit's flags change in, through the queue as a producer (io-pimdir `PimdirSearchTagPlan::stage`), every action passing the capability gate before the first is appended, and SHALL report each placement as `queued` with its row, `unchanged`, `unknown-flags` (its flags were never read), `ambiguous-source`, `unsynced` or `unsupported`, staging nothing for those. `read`, `flagged`, `answered` and `draft` name `\Seen`, `\Flagged`, `\Answered` and `\Draft` as `is:` does; a flag is otherwise checked as `pimdir flag` checks it. `--dry-run` reports the plan and stages nothing. Himalaya SHALL NOT drain the queue: the owner applies the rows.

#### Scenario: Tagging a query
- GIVEN two messages of the index from `alice@x.org`, one already flagged
- WHEN `himalaya pimdir tag from:alice@x.org --add flagged` runs
- THEN one `set-flags` row is queued for the other one, and the flagged one is reported unchanged

### Requirement: A pimdir search says what it did not cover
The output of `pimdir search`, `count` and `tag` SHALL carry the coverage of the answer (SEARCH §8.2): `bodiless`, the messages in scope whose body is not local, matched on their summary alone; `missing`, `unparseable` and `encrypted`, those whose body the index could not read, could not parse, or holds encrypted, indexed from their headers; `unindexed`, those whose last change the index has not folded in; `behind`, how many changes the store drew above the index, store-wide; `since` and `until`, the range below or above which the store is not known to hold every message; `filling`, whether a first round is still listing a mailbox in scope; and `complete`, as io-pimdir's `is_complete` answers it. `bodiless`, `missing`, `unparseable` and `encrypted` SHALL be the counts of the scope whatever the query, and SHALL leave the answer incomplete only while a term reads what they hide: a `bodiless`, a `missing` or an `unparseable` body while a term reads mail bodies, `thread:` among them; a `bodiless` one under `has:attachment` too, a message holding no body answering it by a mark its body corrects; an `encrypted` one while a text term is matched in a body or an attachment. A mailbox listed and not fetched so answers a flag, a date or an address whole. Himalaya SHALL take what a term reads from io-pimdir (`PimdirSearchLocal::reads`, `PimdirSearchReads::counts_bodiless`) and restate none of it. The coverage carries no zone member: a mail date is an instant, which the index's zone does not read. The plain output SHALL print one line under its list naming what left the answer incomplete, a count no term reads left out, and nothing when the answer is complete; an index behind the store is named with how far and that the sync engine refreshes it.

#### Scenario: An unparseable body under a header query
- GIVEN a pimdir mailbox holding one message whose body no parser accepted
- WHEN `himalaya --json pimdir search from:alice@x.org` runs
- THEN the coverage carries `"unparseable":1` and `"complete":true`, and the plain output prints no `Incomplete:` line
- AND `himalaya --json pimdir search body:invoice` over the same mailbox carries `"complete":false`

#### Scenario: A body not local under a flag query
- GIVEN a pimdir mailbox whose round is closed, holding one message whose body is not local
- WHEN `himalaya --json pimdir search is:flagged` runs
- THEN the coverage carries `"bodiless":1` and `"complete":true`, and the plain output prints no `Incomplete:` line
- AND `subject:lunch`, `thread:a@x.org` and `has:attachment` over the same mailbox each carry `"complete":false`, the `Incomplete:` line naming the message with no local body

#### Scenario: Mail before the coverage
- GIVEN a pimdir mailbox listed since 2026-03-01
- WHEN `himalaya pimdir search date:2026-01-01..2026-02-01` runs
- THEN the coverage says mail before 2026-03-01 is not synced, and `complete` is false

### Requirement: A missing or outdated pimdir index is a coded error
A store with no search index SHALL fail every index command with the code `index-missing`, an index of this draft another build, tokenizer or store built with `index-outdated`, and an index of another draft or shape (pimdir SEARCH §2) with `index-version`, each saying the store's sync engine builds or rebuilds it; one of another shape or a later draft is left as it is. None SHALL advise deleting anything, and Himalaya SHALL NOT rebuild any of them.

#### Scenario: No index yet
- GIVEN a pimdir store whose sync engine never built its index
- WHEN `himalaya --json pimdir search hello` runs
- THEN the command exits 1 with the code `index-missing`

#### Scenario: An index of an earlier draft
- GIVEN a pimdir store whose `index_meta.format_version` is `search-draft-11`
- WHEN `himalaya --json pimdir search lunch` runs
- THEN the command exits 1 with the code `index-version`, and the index is left as it is

### Requirement: A refused pimdir query names its fault
A query the SEARCH §8.4 rules refuse SHALL be refused whole with the code `query-refused`, nothing searched. A query past the bounds of SEARCH §8.3, more than 1,024 Unicode scalar values, more than 32 groups open at once or more than 128 terms, SHALL be refused so, its §8.4 code `limit-exceeded`. The plain output SHALL render the fault's span over the query with Himalaya's ariadne setup, under the one `Error:` the printer heads it with; the JSON error's message SHALL name the §8.4 code, the span as Unicode scalar offsets and what was expected there.

#### Scenario: An unknown field
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search frm:alice` runs
- THEN the command exits 1 with the code `query-refused` and a message naming `unknown-field` at `0..4`

#### Scenario: A query past its bounds
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search` runs a query of 129 terms
- THEN the command exits 1 with the code `query-refused` and a message naming `limit-exceeded` at the 129th term's span
