---
cairn: tasks
change: pimdir-search-reads-local-days
---

# Tasks

- [x] io-pimdir carrying `PimdirSearchClock::zone` (pimdir-follows-storage-draft-09)
- [x] src/pimdir/index.rs: `local_clock`, `count_mail` and `tag_mail` taking the clock; search.rs, count.rs and tag.rs stating it
- [x] Test: a message at 00:30 Europe/Paris found by its local day, `today` and not `yesterday` nor the previous day; fails with the clock's zone `None` (UTC)
- [x] `pimdir search` help names the local zone
- [x] Delta, fold into spec/search.md, CHANGELOG; log
