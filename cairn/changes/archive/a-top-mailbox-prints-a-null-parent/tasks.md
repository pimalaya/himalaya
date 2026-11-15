---
cairn: tasks
change: a-top-mailbox-prints-a-null-parent
---

# Tasks

- [x] src/pimdir/mailbox/create.rs: drop `skip_serializing_if` from `parent`.
- [x] Test `a_mailbox_at_the_top_prints_a_null_parent`: the JSON holds `"parent": null` and the output schema requires `parent`; it fails with the attribute back.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; the log entry.
