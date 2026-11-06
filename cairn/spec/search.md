---
cairn: spec
capability: search
status: current
---

# Search

Shared envelope search is expressed in one backend-agnostic query language, himalaya's own, parsed once and translated per backend. The query type carries a filter tree and a sort list; the parser is built on chumsky and renders its errors through the CLI's ariadne setup. Each backend matches text its own way, and refuses a clause it cannot search rather than dropping it.

A pimdir account also answers the pimdir SEARCH query language natively, from the store's search index, under `himalaya pimdir`; the shared language stays himalaya's.

### Requirement: One query language
The `envelope search` query SHALL parse into a single `SearchEmailsQuery` (filter plus sort), independent of the backend. The grammar covers the `date`, `after` and `before` date clauses, `after` and `before` both strict, the `from`, `to`, `cc` and `bcc` address clauses and `address` over all four, the `subject` and `body` text clauses, the `flag` clause over any flag name and the valueless `attachment` clause, combined with `not`, `and` and `or`, binding in that order, grouped with parentheses, and an `order by` list over date, from, to and subject.

### Requirement: The query's clauses are listed on -h
The first paragraph of the `QUERY` argument's help, which `-h` shows, SHALL name every clause, operator and the sort, comma separated, and point at `--help`, whose remaining paragraphs give each clause's syntax, examples and the precedence. The command's long help SHALL say how each backend matches text.

### Requirement: A query that does not parse is refused with a code
A query the grammar refuses SHALL fail with the code `search-invalid`, the plain output rendering the parser's diagnostics over the query with ariadne.

### Requirement: A clause a backend does not answer is refused with a code
A query holding a clause the account's backend cannot compile SHALL be refused with the code `search-unsupported`; nothing is searched and no clause is dropped. A clause or sort key the backend declares `none` SHALL be refused before connecting, naming the option and the backend; one it answers in part SHALL be refused while it is translated, before any request, the message quoting the clause as the language writes it and saying why. IMAP refuses `attachment`, Gmail `flag answered` and keyword flags, Microsoft Graph every flag clause.

### Requirement: What a backend searches is declared
Each backend SHALL declare, per clause and per sort key, an `envelope.search.<option>` capability (`envelope.search.from`, `envelope.search.sort.date`), and its `envelope.search` level SHALL say how it matches text: IMAP and the local evaluator by case-insensitive substring (`full`); JMAP as the server defines `text` and the header conditions (RFC 8621 section 4.4.1), Gmail by words as its search box does, Graph by KQL words in relevance order (`partial`). Gmail's and JMAP's date clauses are `partial`, comparing the received date; Gmail's `body` matches headers too; sorting on Gmail and Graph orders the fetched page only; pimdir's `body` misses a body not local yet.

### Requirement: Per-backend translation
Each backend SHALL translate the shared query into its native surface. IMAP translates to `UID SEARCH` / `UID SORT` keys (`SENTBEFORE` for `before`, an `OR` of `FROM`, `TO`, `CC` and `BCC` for `address`), with a client-side sort fallback when the server lacks `SORT`. JMAP translates to a `JmapFilter` and comparator list scoped to the mailbox id; on the chain of `and` from the root a date clause over-approximates on the received date and is re-checked exactly on the sent date client-side, while under `or` or `not` it reads the received date alone, a client-side residual being unable to hold across a disjunction or a negation. Maildir, m2dir and mbox list then evaluate a shared client-side matcher (`matches_filter` plus `sort_envelopes`) over the fetched envelopes, reusing the read bytes for body clauses and reading the `Cc:` and `Bcc:` headers off them only when a clause needs them. pimdir evaluates the same matcher over envelopes built from the stored summaries, the copy recipients and the attachment mark coming from the summary, reading a message's local body only when the filter holds a body clause; a message whose body is not local matches no body clause. Gmail translates to a `q` value, a date clause as Unix seconds bounding the UTC day (`after d` from the start of the next day, one second early so a boundary message is kept, `before d` to the start of that day), Gmail reading a bare date as midnight in PST and comparing it with the message's received date. Graph translates to a KQL `$search` (`participants` for `address`, `hasAttachments` for `attachment`) and sorts the page locally, Graph refusing `$skip` and `$orderby` beside it.

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
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-draft-09 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. The answer SHALL be the store as of the owner's last refresh: a queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort date|name|relevance` orders the hits (date by default), `--page` and `--page-size` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date, and its hits. Under `--json` the page carries `more`, whether hits or threads follow it.

A `date:` term's years, months and days, `today` and `yesterday` SHALL be read in the local zone (SEARCH §8.1), in `pimdir search`, `count` and `tag` alike: `TZ`, then the system's zone, by its IANA name. A zone with no IANA name, a POSIX `TZ` string or a system zone that cannot be named, or no zone at all, SHALL leave them in UTC with one warning.

#### Scenario: A body match
- GIVEN a pimdir store whose index holds a message whose body says `quarterly`
- WHEN `himalaya pimdir search quarterly` runs
- THEN the message is listed with `matched` `body` and a snippet holding `quarterly`

#### Scenario: A staged message is not listed
- GIVEN a message added to a pimdir mailbox and still in the queue
- WHEN `himalaya pimdir search` runs a query its body matches
- THEN it is not listed until the owner applies the add and refreshes the index

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
The output of `pimdir search`, `count` and `tag` SHALL carry the coverage of the answer (SEARCH §8.2): `bodiless`, the messages in scope whose body is not local, matched on their summary alone; `unindexed`, those whose last change the index has not folded in; `behind`, how many changes the store drew above the index, store-wide; `since` and `until`, the range below or above which the store is not known to hold every message; `filling`, whether a first round is still listing a mailbox in scope; and `complete`, true only when none of these holds. The plain output SHALL print one line under its table naming what did not hold, and nothing when the answer is complete; an index behind the store is named with how far and that the sync engine refreshes it.

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
- GIVEN a pimdir store whose `index_meta.format_version` is `search-draft-08`
- WHEN `himalaya --json pimdir search lunch` runs
- THEN the command exits 1 with the code `index-version`, and the index is left as it is

### Requirement: A refused pimdir query names its fault
A query the SEARCH §8.4 rules refuse SHALL be refused whole with the code `query-refused`, nothing searched. A query past the bounds of SEARCH §8.3, more than 1,024 Unicode scalar values, more than 32 groups open at once or more than 128 terms, SHALL be refused so, its §8.4 code `limit-exceeded`. The plain output SHALL render the fault's span over the query with Himalaya's ariadne setup; the JSON error's message SHALL name the §8.4 code, the span as Unicode scalar offsets and what was expected there.

#### Scenario: An unknown field
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search frm:alice` runs
- THEN the command exits 1 with the code `query-refused` and a message naming `unknown-field` at `0..4`

#### Scenario: A query past its bounds
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search` runs a query of 129 terms
- THEN the command exits 1 with the code `query-refused` and a message naming `limit-exceeded` at the 129th term's span
