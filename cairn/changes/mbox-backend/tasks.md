---
cairn: change-tasks
id: mbox-backend
status: landed
---

- [x] io-mbox: scaffold on io-maildir conventions (no_std, `client`/`parser`/`serde`, cairn, dual license).
- [x] io-mbox: `From_` line, quoting per variant, flag codec, streaming scanner with content-hash ids.
- [x] io-mbox: `MboxIndex` reuse/resume/rebuild.
- [x] io-mbox: dotlock + fcntl lock coroutines, std client with OFD locks.
- [x] io-mbox: entry get/append/update/copy/move, mailbox create/delete/rename/list.
- [x] io-mbox: unit, integration, stress, interop (formail, GNU mail) and corpus tests, randomized scanner test, fuzz target (92% tarpaulin, the rest `Display` impls and defensive arms).
- [x] io-mbox: store appended messages with LF line endings (found here, see its append-lf-line-endings change).
- [x] himalaya: `mbox` feature, `MboxConfig`, `Backend::Mbox`, account check/list arms, JSON Schemas.
- [x] himalaya: `src/mbox/` protocol commands and shared backend adapter, XDG index and envelope cache.
- [x] himalaya: scratch-copy end-to-end over public archives, GNU mail and formail interop after every write. Concurrent delivery is covered by io-mbox's stress test.
- [x] Docs: config.sample.toml, README, CONTRIBUTING, CHANGELOG.
- [x] Fold the delta into the specs, write the landing log.
