---
cairn: tasks
change: pimdir-queue-row
---

- [x] Backend: `add_message` returns `PimdirStaged`, `send_message` `PimdirSubmitted`, `queue_row` finds a pending or parked row of the account
- [x] `pimdir message add`, `pimdir message send`, `pimdir queue show`, their JSON Schemas
- [x] Tests: rows named on add and send, found while pending, gone once cancelled
- [x] Fold into spec/backends.md, log, CHANGELOG
- [ ] `applied` with `seq` once io-pimdir records the row's outcome
