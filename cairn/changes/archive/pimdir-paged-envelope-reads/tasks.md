---
cairn: tasks
change: pimdir-paged-envelope-reads
---

- [x] `envelope list`: keyset walk to the end of the page asked for
- [x] `envelope search`: early stop in the default order, date bounds on the walk, unread chip when the queue is quiet, fallback to the whole-mailbox read
- [x] Tests: old and new reads agree on a seeded store (dates, undated, flags, queued actions, page boundaries, page beyond the end)
- [x] Timed comparison on ~50,000 mails (ignored by default)
- [x] Fold into spec/backends.md, log, CHANGELOG
