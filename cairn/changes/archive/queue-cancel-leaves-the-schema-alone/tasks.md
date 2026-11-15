---
cairn: tasks
change: queue-cancel-leaves-the-schema-alone
---

- [x] Test first, failing on `cancel_action`: a cancel on a store whose schema differs from the canonical one changes nothing in the schema and is refused
- [x] Test first, failing on the unscoped cancel: a row anchored on another account's collection is refused and stays queued
- [x] `cancel_queued`: `action_status` scoped to the account, then `open_current` and `drop_action`; `NotCurrent` refused naming the sync engine
- [x] Prompt and output name the action kind; plain output ends with a newline
- [x] Fold into spec/backends.md, log, CHANGELOG
