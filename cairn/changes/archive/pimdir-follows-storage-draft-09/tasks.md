---
cairn: tasks
change: pimdir-follows-storage-draft-09
---

# Tasks

- [x] `io-pimdir = "0.9"`, patch rev 5c18a02; mime-rs 0.1 from crates.io; the ical-rs patch dropped
- [x] Build and tests green on the new pin (no compile change needed)
- [x] The `store-version` message checked against a draft-08 store; tests stamping another draft moved to draft-08 and draft-10
- [x] Feature builds: each backend alone, pimdir alone
- [x] Delta, fold into spec/search.md, CHANGELOG; log
- [ ] End to end on a scratch store a draft-09 neverest synced (waits for neverest on 5c18a02)
