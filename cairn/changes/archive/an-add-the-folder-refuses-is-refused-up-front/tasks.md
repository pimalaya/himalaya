---
cairn: tasks
change: an-add-the-folder-refuses-is-refused-up-front
---

# Tasks

- [x] src/pimdir/backend.rs test `an_add_where_the_source_adds_nothing_is_refused_up_front`: a store whose `msgraph` source declares `mail.message.add` `none` source-wide and `full` on `msgraph/Drafts`; an add to `msgraph/Archive` fails `unsupported` naming the mailbox and the detail, no row and no body left, an add to `msgraph/Drafts` is queued.
- [x] Fold into cairn/spec/backends.md; the log entry.
