---
cairn: tasks
change: pimdir-root-shell-expand
---

- [x] `PimdirClient::new` expands `~`/env vars on `root` (via `shellexpand::full`)
      before opening the store and blobs.
- [x] `config.sample.toml` documents `pimdir.root` (and that `source` is
      auto-detected, not normally set).
- [x] `PimdirConfig.source` doc corrected (auto-detected, not "defaults to local").
- [x] Build + fmt clean.
- [x] Verified read-only against a real Neverest store: mailbox list, envelope
      list, message read.
- [ ] Fold delta into `cairn/spec/backends.md`; write log entry.
