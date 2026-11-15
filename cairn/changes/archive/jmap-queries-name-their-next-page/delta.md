---
cairn: delta
change: jmap-queries-name-their-next-page
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Listings resume from their cursor
In commands.md, it ends:

`jmap email query`, `jmap mailbox query` and `jmap submission query` SHALL print `nextPage` beside their collection the same way, a page following while the query's `total` lies past it, or, on a server computing none, while it comes back full, and SHALL take it back through `--page-token`, a token that is no page number from 1 being refused; `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create`, printing the same shape, SHALL print `nextPage` as `null`.

It gains the scenario *A JMAP query reaching its total*.

### Requirement: Text output is plain text
The `jmap email query|get` item reads "A query is paged by number.", the rule of a page following moving to *Listings resume from their cursor*.

## REMOVED Requirements

None.
