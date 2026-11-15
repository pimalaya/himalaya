---
cairn: change
id: jmap-queries-name-their-next-page
status: landed
created: 2026-11-15
---

# The JMAP queries name their next page

## Why

Task H14 of the family's cli-alignment-2026-11-15 plan, raised by H1's agent. `jmap email query`, `jmap mailbox query` and `jmap submission query` printed `Next page:` in text whenever a page came back full, an exactly full last page included, and no `nextPage` key, against pimalaya-cli's cairn/spec/output.md ("Paging resumes") and what H1 made `envelope list` do.

## What

The three queries print `nextPage`, the next page number while the query's `total` lies past the page (`calculateTotal` asked, as io-jmap does for every query), a full page standing for more on a server computing none, `null` otherwise, and take it back through pimalaya-cli's `--page-token`, winning over `--page`. Their text prints `Next page:` from it. `jmap email get`, `jmap mailbox get`, `jmap submission get` and `jmap submission create` share the output types and print `nextPage` `null`. Not breaking: a key and a flag added, a wrong `Next page:` line gone.
