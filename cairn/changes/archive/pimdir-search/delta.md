---
cairn: delta
change: pimdir-search
---

## ADDED Requirements

### Requirement: pimdir searches through the store's index
`himalaya pimdir search <query>...` SHALL answer the pimdir SEARCH query language (search-draft-05 §8) from the store's search index, scoped to the account's mail: the query is read as `(<query>) kind:mail`, with `account:<account>` when the store groups the account under a name. The words of the query are joined by spaces into one query. Himalaya SHALL open the index read-only (io-pimdir `PimdirIndex::open_query`) and SHALL NOT create, rebuild nor refresh it: the store's owner, its sync engine, builds and refreshes it. The answer SHALL be the store as of the owner's last refresh: a queued write is not listed nor folded in.

Each hit SHALL carry the shared envelope's keys (its `id` the public `seq`, which `message read` takes with one of its mailboxes), the account's `mailboxes` holding it, what a text term matched (`body`, `summary`, or `null` when only field terms did) and a `snippet` of its body. `--sort date|name|relevance` orders the hits (date by default), `--page` and `--page-size` page them, and `--threads` collapses them to their threads (SEARCH §9), each with its id, the number of messages it holds in the account, its first and last date, and its hits. Under `--json` the page carries `more`, whether hits or threads follow it.

#### Scenario: A body match
- GIVEN a pimdir store whose index holds a message whose body says `quarterly`
- WHEN `himalaya pimdir search quarterly` runs
- THEN the message is listed with `matched` `body` and a snippet holding `quarterly`

#### Scenario: A staged message is not listed
- GIVEN a message added to a pimdir mailbox and still in the queue
- WHEN `himalaya pimdir search` runs a query its body matches
- THEN it is not listed until the owner applies the add and refreshes the index

### Requirement: pimdir counts, threads and tags from the index
`himalaya pimdir count <query>...` SHALL answer how many messages the scoped query matches, with its coverage. `himalaya pimdir thread <id>` SHALL list the thread of one message, oldest first, a message no thread holds being its own. `himalaya pimdir tag <query>... --add <flag> --remove <flag>` SHALL stage one `set-flags` per placement each hit's flags change in, through the queue as a producer (io-pimdir `PimdirSearchTagPlan::stage`), every action passing the capability gate before the first is appended, and SHALL report each placement as `queued` with its row, `unchanged`, `unknownFlags` (its flags were never read), `ambiguousSource`, `unsynced` or `unsupported`, staging nothing for those. `read`, `flagged`, `answered` and `draft` name `\Seen`, `\Flagged`, `\Answered` and `\Draft` as `is:` does; a flag is otherwise checked as `pimdir flag` checks it. `--dry-run` reports the plan and stages nothing. Himalaya SHALL NOT drain the queue: the owner applies the rows.

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
A store with no search index SHALL fail every index command with the code `index-missing`, and an index another version built with `index-outdated`, each saying the store's sync engine builds or rebuilds it. Neither SHALL advise deleting anything.

#### Scenario: No index yet
- GIVEN a pimdir store whose sync engine never built its index
- WHEN `himalaya --json pimdir search hello` runs
- THEN the command exits 1 with the code `index-missing`

### Requirement: A refused pimdir query names its fault
A query the SEARCH §8.4 rules refuse SHALL be refused whole with the code `query-refused`, nothing searched. The plain output SHALL render the fault's span over the query with Himalaya's ariadne setup; the JSON error's message SHALL name the §8.4 code, the span as Unicode scalar offsets and what was expected there.

#### Scenario: An unknown field
- GIVEN any pimdir account with an index
- WHEN `himalaya --json pimdir search frm:alice` runs
- THEN the command exits 1 with the code `query-refused` and a message naming `unknown-field` at `0..4`

## MODIFIED Requirements

### Requirement: The JSON contract is described and versioned
The code table gains `store-foreign`, `index-missing`, `index-outdated`, `index-fts5-missing`, `query-refused`, `query-unanswerable` and `tag-refused` (folded into commands.md).
