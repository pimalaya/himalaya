---
cairn: tasks
change: pimdir-search
---

- [x] Phase 0, the interim body clause in the shared query: a-body-clause-reads-the-local-body
- [x] io-pimdir's query-only `PimdirIndex::open_query` and lag report (`PimdirSearchLocal::behind`) landed; neverest indexing at sync time is neverest's own change, in progress on 2026-10-30, and nothing Himalaya waits on to build
- [x] `pimdir` feature enables io-pimdir `search`
- [x] `pimdir search <query>...` with `--sort`, `--page`, `--page-size`, `--threads`; table and JSON (hits, `more`, coverage)
- [x] `pimdir thread <id>`, `pimdir count`, `pimdir tag --add|--remove [--dry-run]` through the queue and the capability gate
- [x] SEARCH §8.4 refusals coded `query-refused`, spans rendered with ariadne
- [x] Coverage line; `index-missing` and `index-outdated` codes
- [x] Integration tests on an indexed scratch store; benchmark query time
- [x] spec/search.md, commands.md, backends.md, README pimdir section, CHANGELOG; fold, log
- [x] Retire the draft a-search-says-what-it-covered into this change (absorption approved with the plan on 2026-10-30)
