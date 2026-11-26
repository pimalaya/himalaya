---
cairn: tasks
change: a-search-says-what-it-covered
---

- [x] ~~src/pimdir/backend.rs: the coverage of a search (bodiless candidates for a body clause, the range against the mailbox's coverage and open round)~~ Dropped: the change was abandoned on 2026-10-30, absorbed into pimdir-search, where `pimdir search`, `count` and `tag` carry io-pimdir's coverage and the shared `envelope list` carries none.
- [x] ~~Table output: one line when incomplete; `--json`: a `coverage` object in camelCase~~ Dropped: landed as the `Incomplete:` line and the `coverage` object of `pimdir search` (pimdir-search, 2026-10-30), not on the shared output.
- [x] ~~Move onto io-pimdir's coverage type once search-feature lands~~ Dropped: pimdir-search reads io-pimdir's coverage type from the start.
- [x] ~~Tests: a body clause over a headers-only mailbox; a date range below the coverage; a complete search printing nothing extra~~ Dropped: the coverage is tested where it landed, under pimdir-search.
- [x] ~~Spec: search.md and backends.md; fold, log, CHANGELOG~~ Dropped: abandoned, nothing to fold; pimdir-search folded and logged the coverage on 2026-10-30.
