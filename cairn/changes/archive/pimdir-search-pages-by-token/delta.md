---
cairn: delta
change: pimdir-search-pages-by-token
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir searches through the store's index
In search.md, its second paragraph ends:

Under `--json` the page SHALL print `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`: `nextPage` the number of the next page, as a string, while hits or threads follow the page, `null` once the answer is complete, which `--page-token` SHALL take back, winning over `--page`, a token that is no page number from 1 being refused. A hit whose message the store cannot read SHALL be an `errors` entry carrying its id, coded `message-unreadable` unless the store names its own code, the other hits listed (a thread without that hit) and the exit status 3; in text the entries print after the items, as every list prints them.

It gains the scenarios *A second page of a search* and *A hit the store cannot read*.

### Requirement: A command acting on several items lists what failed
In commands.md, `pimdir search` joins the commands printing `errors`. In the `message-unreadable` row of the code table, a message whose row the store could not read (`pimdir search`) joins the causes.

## REMOVED Requirements

None.
