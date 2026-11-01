---
cairn: tasks
change: pimdir-adopts-the-io-pimdir-release
---

- [x] Build on io-pimdir's release API, through the local checkout until its changes of 2026-10-30 are pushed (the git rev, the requirement bump and the pins moved to v2-release)
- [x] Store errors carry codes from `PimdirError::code()`; no "delete the store" advice; a newer store says upgrade himalaya; tests for a stale, a newer and an owned store
- [x] Duplicate creates refused at staging (a link id the mailbox holds, a create already pending); test
- [x] `add_message` and `send_message` through `PimdirProducer::stage_body`
- [x] `submit` built from `PimdirSubmit`; `SubmitPayload` and `SUBMIT` removed
- [x] `account check` checks a pimdir block (found reading the code, reproduced: a pimdir-only account was refused as declaring no backend)
- [x] commands.md code table, backends.md, log, CHANGELOG
