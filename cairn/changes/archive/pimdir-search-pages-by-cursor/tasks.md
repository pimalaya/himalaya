---
cairn: tasks
change: pimdir-search-pages-by-cursor
---

# Tasks

- [x] Cargo.toml, Cargo.lock: io-pimdir 0.16.0.
- [x] src/pimdir/index.rs: pages carry `next`, a `PimdirSearchCursor`.
- [x] src/pimdir/search.rs: `PimdirSearchPlace` fingerprints its place, `token` and `cursor`; `--page` removed; text pages by token.
- [x] Tests: `a_page_token_is_bound_to_its_query` (unit), tests/pimdir_search.rs `a_token_of_another_query_is_refused`, failing on e440b4e4 (the page number `2` of another query was taken).
- [x] Fold into cairn/spec/search.md; CHANGELOG; MIGRATION (development build).
