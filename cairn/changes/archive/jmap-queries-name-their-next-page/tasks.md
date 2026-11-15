---
cairn: tasks
change: jmap-queries-name-their-next-page
---

# Tasks

- [x] `query_page_follows` (src/jmap/backend.rs), shared with the unified listing's `query_follows`.
- [x] The three query commands take `PageTokenArg` and print `nextPage`; their outputs print `Next page:` from it; the gets and `submission create` print `null`.
- [x] Tests: `a_query_page_follows_below_its_total`, `a_full_page_names_the_next_only_when_one_follows` (mailbox), `a_full_last_page_names_no_next_page` (email).
- [x] Fold into cairn/spec/commands.md; CHANGELOG; the log entry.
