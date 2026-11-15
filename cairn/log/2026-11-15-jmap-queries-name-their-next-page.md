---
cairn: log
change: jmap-queries-name-their-next-page
landed: 2026-11-15
---

# The JMAP queries name their next page

Task H14 of the family's cli-alignment-2026-11-15 plan, left over by H1.

## What landed

- src/jmap/backend.rs: `query_page_follows`, whether a page follows a JMAP query's page (below its `total`, else after a full page, never at a page size of 0), which `query_follows` now calls.
- src/jmap/email/query.rs, mailbox/query.rs, submission/query.rs: the three queries take `--page-token` (pimalaya-cli `PageTokenArg`, winning over `--page`), print `nextPage` and write `Next page:` from it; `jmap email query` asks `calculateTotal`, as the mailbox and submission queries of io-jmap always do. The gets and `jmap submission create`, sharing the outputs, print `nextPage` `null`.
- Tests: `a_query_page_follows_below_its_total`, `a_full_page_names_the_next_only_when_one_follows` and `a_full_last_page_names_no_next_page`, the last two failing on the old rule, which printed `Next page:` after any full page.
- CHANGELOG.

## Capabilities moved

- [commands](../spec/commands.md): *Listings resume from their cursor* names the three queries and gains a scenario; *Text output is plain text*, the `jmap email query|get` item.
