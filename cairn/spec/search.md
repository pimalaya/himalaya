---
cairn: spec
capability: search
status: current
---

# Search

Unified envelope search reads the pimdir SEARCH query language (§8), parsed by io-pimdir (`PimdirQuery`), the language `pimdir search`, Calendula and Cardamum read. A query is lowered once to a filter tree the server translations read; Maildir and mbox match it with io-pimdir's in-memory mail matcher (`search::mail`), as an index holding the messages answers. Each backend matches text its own way, and refuses a field it cannot search rather than dropping it. The order is a flag, never part of the query.

### Requirement: One query language
The `envelope search` query SHALL be pimdir SEARCH §8, its words joined by spaces and parsed by `PimdirQuery::parse`; `envelope search` with no query lists the mailbox. The order SHALL be `--sort` (`newest-first`, `oldest-first`, `title`, `sender`, `relevance`, SEARCH §8.2), the backend's own when omitted. A field outside mail (`kind:`, `status:`, `priority:`, `category:`, `person:`, `location:`, `account:`, `collection:`, `folder:`, `changed:`, `thread:`, `id:`, `linked:`, `email:`, `organizer:`, `attendee:`, `is:encrypted`, `has:body` and the card marks, `attachment:`, `label:`, `property:`) SHALL be refused with `search-unsupported` before any backend is reached.

### Requirement: A reserved form is refused
A text or a value beginning with `~`, one of two characters or more beginning with `/`, and a value beginning with `{` (pimdir SEARCH §8.3) SHALL refuse the whole query, its §8.4 code `reserved-form`: `search-invalid` on `envelope search` whatever the backend, `query-refused` on `pimdir search`, `count` and `tag`. Quoted, the characters are searched.

#### Scenario: A tilde word
- WHEN `himalaya --json envelope search '~draft'` runs on a Maildir account
- THEN it exits 1 coded `search-invalid`, the message naming `reserved-form`, and `envelope search '"~draft"'` lists the messages holding the word

### Requirement: Labels and properties are read from the store
`pimdir search`, `count` and `tag` SHALL answer `label:` and `property:<key>=<value>` as io-pimdir answers them, from the store's item properties (pimdir STORAGE §4.3), never from the index. `envelope search` SHALL refuse both with `search-unsupported`, as a field outside mail. Himalaya stages no `set-property`.

### Requirement: Dates are read against one clock in the local zone
A `date:` or `when:` term SHALL resolve as SEARCH §8.1 says (`PimdirQueryDateRange::bounds`), against the clock read once per query and the local zone, `TZ` then the system's. A side a calendar point (a year, a month, a day, `today`, `yesterday`) names SHALL be lowered to a day, a range of one day to that day; a side an instant or a relative amount names to an instant, its start inclusive and its end exclusive to the second. IMAP SHALL serve an instant on the chain of `and` from the root alone, every message of the three-day band around it searched then kept on its side; under `or` or `not` it SHALL refuse it with `search-unsupported`. Microsoft Graph, whose KQL `sent` compares days, SHALL refuse an instant with `search-unsupported`, a relative amount (`date:2d`) included, the capability check refusing `date.instant` and the translation refusing the lowered clause with one reason, which names a day to write instead (`date:2026-11-24..`).

#### Scenario: Five hours ago is an instant
- GIVEN it is 2026-11-14T15:00 in the local zone, and an IMAP mailbox holding a message sent at 09:00 and one at 11:00 that day
- WHEN `himalaya envelope search date:5h` runs
- THEN only the message sent at 11:00 is listed

#### Scenario: An amount of days on Graph
- WHEN `himalaya --json envelope search date:2d` runs on a Microsoft Graph account
- THEN it exits 1 coded `search-unsupported`, nothing searched, the message ending as `date:5h`'s does, `` Microsoft Graph search compares days alone; give a day (`date:2026-11-24..`) or `today` ``

#### Scenario: An instant under or on IMAP
- WHEN `himalaya envelope search date:5h or from:alice` runs on an IMAP account
- THEN it fails with `search-unsupported`, nothing searched

### Requirement: The query's fields are listed on -h
The first paragraph of the `QUERY` argument's help, which `-h` shows, SHALL say the query is pimdir SEARCH §8 and name its operators, and point at `--help`, whose remaining paragraphs list the fields mail answers, the date forms and the zone. The command's long help SHALL say how each backend matches text, a prefix and a phrase, and which date a date term compares: the `Date:` header, but the received date on Gmail and on JMAP under `or` or `not`. A word opening on a hyphen goes after `--`.

### Requirement: A query that does not parse is refused with a code
A query SEARCH §8.4 refuses SHALL fail with the code `search-invalid`: drawn with ariadne over the query in plain output, its §8.4 code, the span and what was expected, the field names for `unknown-field`; under `--json` one line, `Query refused, <code> at <start>..<end>: <message>`.

### Requirement: A field a backend does not answer is refused with a code
A query holding a field or an order the account's backend cannot compile SHALL be refused with the code `search-unsupported`; nothing is searched and no term is dropped. One the backend declares `none` SHALL be refused before connecting, naming the option and the backend; one it answers in part SHALL be refused while it is translated, before any request, the message quoting the term as the query language writes it and saying why. IMAP refuses `has:attachment`, Gmail `is:answered`, keywords but `$Important` and `$Junk`, and a prefix, Microsoft Graph every flag and a `mid:` term beside any other, JMAP `mid:`, every backend but a pimdir index `--sort relevance`.

### Requirement: What a backend searches is declared
Each backend SHALL declare, per field and per order, an `envelope.search.<option>` capability named as io-pimdir names the predicate (`PimdirQueryPredicate::name`: `envelope.search.text`, `title`, `body`, `from`, `to`, `cc`, `bcc`, `with`, `is.read`, `is.flagged`, `is.answered`, `is.draft`, `tag`, `has.attachment`, `date`, `date.instant`, `mid`) and the orders prefixed `sort.` (`envelope.search.sort.newest-first`). Its text fields SHALL say how it matches text: the matcher by words, as SEARCH §8.5 does, and Graph by KQL words, a prefix and a phrase served (`full`); Gmail by words and phrases, a prefix refused (`partial`); IMAP and JMAP as the server matches it, RFC 3501 asking for a substring, a prefix sent as its stem and a phrase as one string (`partial`). Gmail's and JMAP's dates are `partial`, comparing the received date; Gmail's `body` matches headers too, and its `has:attachment` counts inline images (`partial`); sorting on Gmail and Graph orders the fetched page only; IMAP's `sender` order sorts by the mailbox of the first `From:` address (RFC 5256).

### Requirement: Per-backend translation
Each backend SHALL translate the shared query into its native surface. The lowered tree SHALL carry on each text and address leaf whether its text is a prefix (a trailing `*`) or a phrase (quoted), and no translation SHALL answer either as a plain word. IMAP and JMAP SHALL send a prefix as its stem and a phrase as one string, the server matching them as it matches text. IMAP translates to `UID SEARCH` / `UID SORT` keys (`SENTBEFORE` for `before`, an `OR` of `FROM`, `TO`, `CC` and `BCC` for `address`), negations pushed down to single keys (double negations cancelled, De Morgan over `and` and `or`, a negated `address` sent as `NOT FROM x NOT TO x NOT CC x NOT BCC x`), a root `and` sent as the list of its keys, a `flag` naming an IANA keyword sent in its canonical spelling (`flag junk` as `KEYWORD $Junk`), and a UID the server's `SORT` lists twice kept once, with a client-side sort fallback when the server lacks `SORT` or is iCloud (provider-quirks), which orders as RFC 5256 defines (base subject, first address's mailbox, ties in sequence order). JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id; on the chain of `and` from the root a date clause over-approximates on the received date, a day bounded from below by its first instant in any offset (10:00 UTC the day before, at UTC+14:00) so a message dated in an offset east of UTC is found on its own day, and is re-checked exactly on the sent date client-side, while under `or` or `not` it reads the received date alone, a client-side residual being unable to hold across a disjunction or a negation. Maildir and mbox list then match each message with io-pimdir's mail matcher (`PimdirMail::read` over its bytes and its flags in their canonical spelling), then `sort_envelopes`. `sort_envelopes` sorts as RFC 5256 `SORT` does: `subject` on the base subject (section 2.1), `from` and `to` on the mailbox of the first address, both under `i;ascii-casemap`, ties keeping their order. pimdir runs the same matcher over each item, `PimdirMail::read` over its local body when a term reads one (free text or `body:`), `PimdirMail::from_summary` over its stored summary and flags otherwise; a message whose body is not local is matched on its summary. Gmail translates to a `q` value, a conjunction as its terms joined by blanks and a disjunction as a brace group, an alternative holding more than one term (a conjunction, the two bounds of a `date` clause, a negated `address` or disjunction) wrapped in parentheses, Gmail reading every blank-separated term inside braces as an alternative of its own; each pattern is a literal: quotes, braces and parentheses become spaces, a standalone `OR` or `AND` is lowercased, anything but a bare word is quoted, so a phrase is quoted whole, and a pattern with no word left is refused with `search-unsupported`, as is a prefix, Gmail having no prefix search; its sort uses the shared `sort_envelopes`; a date clause as Unix seconds bounding the UTC day (`after d` from the start of the next day, one second early so a boundary message is kept, `before d` to the start of that day), Gmail reading a bare date as midnight in PST and comparing it with the message's received date. Graph translates to a KQL `$search` (`participants` for `address`, `hasAttachments` for `attachment`), a phrase one quoted KQL string, a prefix its stem quoted with a trailing `*` (`subject:\"repo*\"`), and any other word a quoted KQL phrase, so a word that is KQL syntax (`OR`, `x:y`, `alpha*`) is matched as a word, a double quote dropped, and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it. A `message-id` clause SHALL match the whole id, compared byte for byte with the envelope's bare `Message-ID:`: IMAP sends `HEADER Message-ID` with the id in angle brackets (`<id>`), which a server matches as a substring or, as iCloud does, on the whole value, and keeps on the chain of `and` from the root only the fetched envelopes carrying the id, a substring match remaining under `or` and `not` (`partial`); Gmail sends `rfc822msgid:`; Graph answers a lone `message-id` clause with a `$filter` on `internetMessageId`, `$filter` taking no `$search` beside it (`partial`); the local evaluator compares the envelope's id; JMAP declares it `none`, io-jmap carrying no `header` condition. IMAP SHALL send an `after` day as `SENTSINCE` of the day after it and a `before` day as `SENTBEFORE` of that day, both keys holding the strictness of the clause. On the chain of `and` from the root a JMAP `before` clause, a day or an instant, SHALL send no bound on the received date, the sent date checked client-side alone.

#### Scenario: A prefix on Gmail
- WHEN `himalaya --json envelope search subject:repo*` runs on a Gmail account
- THEN it exits 1 coded `search-unsupported` before any request, the message quoting `subject:repo*`

#### Scenario: A prefix and a phrase on Graph
- GIVEN a Graph mailbox holding `weekly report` and `beta weekly report`
- WHEN `envelope search subject:repo*` runs, then `envelope search 'subject:"report weekly"'`
- THEN the first lists both messages, and the second neither, the words of a phrase matched together in order

#### Scenario: A pimdir body clause
- GIVEN a pimdir mailbox holding two local bodies, one containing `attached`, and one message whose body is not local
- WHEN `envelope search -m imap/INBOX body:attached` runs
- THEN only the message whose local body holds `attached` is listed, and `not body:attached` lists the two others

#### Scenario: A negated date clause on JMAP
- GIVEN a JMAP mailbox holding a message sent on 2025-12-01
- WHEN `envelope search not date:2026-01-02..` runs
- THEN the message is listed

### Requirement: Graph flag clauses
Graph SHALL refuse a filter holding a flag clause with the code `search-unsupported`, naming the flag: KQL has no flag property and Graph accepts no `$filter` beside `$search`, so dropping the clause would widen the result.

### Requirement: pimdir searches through the store's index
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-v1 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. Text terms SHALL read the index as the owner last refreshed it; flag terms (`is:read`, `is:unread`, `is:flagged`, `is:answered`, `is:draft`, `tag:` naming a system flag) the store's flags as the sync engine last wrote them, even while the index lags (pimdir SEARCH §8.2). A queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort newest-first|oldest-first|title|sender|relevance` orders the hits (newest first by default), `--page-size` and `--page-token` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date as `firstDate` and `lastDate`, and its hits. Under `--json` the page SHALL print `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`: `nextPage` the token of the next page while hits or threads follow the page, `null` once the answer is complete, which `--page-token` SHALL take back: the place of the last hit or thread the page answered (io-pimdir `PimdirSearchCursor`), so the next page starts after it whatever the index gained or lost since, bound to the account, the query, the order and `--threads` it was printed under. A token another account, query, order or listing printed, or one no search printed, SHALL be refused, coded `page-token-invalid`. A page SHALL be reached by its token alone, `pimdir search` taking no page number. A hit whose message the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other hits listed (a thread without that hit) and the exit status 3; in text the entries print after the items, as every list prints them.

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
- WHEN `himalaya --json pimdir search --page-token <nextPage> --sort title report` runs
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
`himalaya pimdir count <query>...` SHALL answer how many messages the scoped query matches, with its coverage. `himalaya pimdir thread <id>` SHALL list the thread of one message, oldest first, a message no thread holds being its own, under `--json` `{thread, count, firstDate, lastDate, hits, errors}`: a member the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other members listed and the exit status 3. `himalaya pimdir tag <query>... --add <flag> --remove <flag>` SHALL stage one `set-flags` per placement each hit's flags change in, through the queue as a producer (io-pimdir `PimdirSearchTagPlan::stage`), every action passing the capability gate before the first is appended, and SHALL report each placement as `queued` with its row, `unchanged`, `unknown-flags` (its flags were never read, `flags` printing `null`), `ambiguous-source`, `unsynced` or `unsupported`, staging nothing for those. `read`, `flagged`, `answered` and `draft` name `\Seen`, `\Flagged`, `\Answered` and `\Draft` as `is:` does; a flag is otherwise checked as `pimdir flag` checks it. `--dry-run` reports the plan and stages nothing. Himalaya SHALL NOT drain the queue: the owner applies the rows.

#### Scenario: A thread member the store cannot read
- GIVEN a pimdir thread of two messages, the store failing to read the row of the reply
- WHEN `himalaya --json pimdir thread <id of the first>` runs
- THEN it prints the first message under `hits` and `"errors":[{"id":"<the reply's id>","code":"message-unreadable",…}]`, and the exit status is 3

#### Scenario: Tagging a query
- GIVEN two messages of the index from `alice@x.org`, one already flagged
- WHEN `himalaya pimdir tag from:alice@x.org --add flagged` runs
- THEN one `set-flags` row is queued for the other one, and the flagged one is reported unchanged

### Requirement: A pimdir search says what it did not cover
The output of `pimdir search`, `count` and `tag` SHALL carry the coverage of the answer (SEARCH §8.2): `bodiless`, the messages in scope whose body is not local, matched on their summary alone; `missing`, `unparseable` and `encrypted`, those whose body the index could not read, could not parse, or holds encrypted, indexed from their headers; `unindexed`, those whose last change the index has not folded in; `behind`, how many changes the store drew above the index, store-wide; `since` and `until`, the range below or above which the store is not known to hold every message; `filling`, whether a first round is still listing a mailbox in scope; and `complete`, as io-pimdir's `is_complete` answers it. `bodiless`, `missing`, `unparseable` and `encrypted` SHALL be the counts of the scope whatever the query, and SHALL leave the answer incomplete only while a term reads what they hide: a `bodiless`, a `missing` or an `unparseable` body while a term reads mail bodies, `thread:` among them; a `bodiless` one under `has:attachment` too, a message holding no body answering it by a mark its body corrects; an `encrypted` one while a text term is matched in a body or an attachment. A mailbox listed and not fetched so answers a flag, a date or an address whole. Himalaya SHALL take the counts a term reads from io-pimdir (`PimdirSearchCoverage::unread_counted`) and restate none of the rule, and SHALL scope a query through `PimdirQuery::scoped` (`kind:mail` and the account, at the top level where the coverage is read). The coverage carries no zone member: a mail date is an instant, which the index's zone does not read. The plain output SHALL print one line under its list naming what left the answer incomplete, a count no term reads left out, and nothing when the answer is complete; an index behind the store is named with how far and that the sync engine refreshes it.

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
A store with no search index SHALL fail every index command with the code `index-missing`, an index of search-v1 another build, tokenizer or store built with `index-outdated`, and an index of a draft, of another version or of another shape (pimdir SEARCH §2) with `index-version`, each saying the store's sync engine builds or rebuilds it; one of another shape or a later version is left as it is. An index of any search draft, search-draft-17 that Himalaya 3.0.0 read among them, is `index-version` until the sync engine rebuilds it. None SHALL advise deleting anything, and Himalaya SHALL NOT rebuild any of them.

#### Scenario: No index yet
- GIVEN a pimdir store whose sync engine never built its index
- WHEN `himalaya --json pimdir search hello` runs
- THEN the command exits 1 with the code `index-missing`

#### Scenario: An index of a draft
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
