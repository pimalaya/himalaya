---
cairn: tasks
change: pimdir-queue-receipts
---

- [x] io-pimdir by git rev `3a9ffb8`
- [x] `queue_row` returns `PimdirActionStatus`, filtered to the account; `queue show` prints the four states
- [x] `create_mailbox` through `enqueue_collection_create`
- [x] Tests: an applied add names its seq; a cancelled row is unknown; the output shapes
- [x] Fold into spec/backends.md, log, CHANGELOG
