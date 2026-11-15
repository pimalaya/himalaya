---
cairn: tasks
change: has-attachment-one-rule
---

# Tasks

- [x] Add `part::marks_attachment` and `part::has_attachment`
- [x] Apply the rule to `BODYSTRUCTURE` on IMAP
- [x] Apply the rule in `envelope::from_raw`, shared by Maildir, m2dir and mbox, and in the local `attachment` search clause
- [x] Version the mbox cache so an older one is rebuilt
- [x] Fill the mark on m2dir `envelope search`
- [x] One test per row of the #802 table on IMAP and through `envelope::from_raw`, with the rows the rule adds
- [x] Fold the delta into the spec, write the log entry, add the changelog line
