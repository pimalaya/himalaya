---
cairn: tasks
change: pimdir-search-pages-by-token
---

# Tasks

- [x] `PimdirSearchCommand` takes `PageTokenArg`; the outputs print `nextPage` and `errors`, no `more`; a non-empty `errors` exits 3.
- [x] `PimdirClient::mail_hits`: a hit `mail_hit` cannot read is an item error, by message and by thread; `pimdir thread` still fails whole.
- [x] Tests: `a_hit_the_store_cannot_read_is_an_item_error` (src/pimdir/index.rs), `a_page_names_the_next_and_its_errors` (src/pimdir/search.rs), tests/pimdir_search.rs through the binary.
- [x] Fold into cairn/spec/search.md and commands.md; CHANGELOG and MIGRATION.md; the log entry.
