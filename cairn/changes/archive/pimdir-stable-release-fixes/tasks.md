---
cairn: tasks
change: pimdir-stable-release-fixes
---

- [x] Test first: `copy`, `move` and `delete` with one known and one unknown id stage the known one and report one; none known stages nothing
- [x] `delete_messages` and `refile` check all, then stage; return the found count
- [x] Test first: a held item whose blob is gone fails with `body-pending`
- [x] Missing blob mapped to `body-pending`
- [x] Test first: a flag add the message already carries queues nothing and counts the message found
- [x] `stage_flags` skips no-op changes; `PimdirFlagged.queue_id` optional; `pimdir flag` output prints unchanged rows
- [x] Trailing newline on `pimdir queue show`, `queue cancel`, `performer`, `message add`, `message send`, `mailbox create`, shared `message add` and `MessageRouteOutput`
- [x] Test first: a parked row of the mailbox shows in `pimdir queue list` with its error
- [x] `pimdir queue list` lists parked rows (`parked` in JSON, a second table in plain)
- [x] Test: a store holding two accounts is refused with `pimdir.account` unset, naming both
- [x] Check: no placeholder id for a queued creation (a create returns the link id, the queue view's ROW is the row id); the queue view's envelope still carries `"id": ""`, moved to a-queued-message-has-a-null-id
- [x] "pimdir SPEC" citations to STORAGE/SYNC in src/ and cairn/spec/; config.sample.toml example account
- [x] CHANGELOG [Unreleased]: pimdir lines as a net diff, no commit hashes, :26 and :49 corrected
- [x] Fold into spec/backends.md, log
