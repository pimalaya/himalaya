---
cairn: tasks
change: a-pimdir-file-names-its-part
---

# Tasks

- [x] src/shared/message/part.rs: `Leaf::section` readable.
- [x] src/pimdir/attachment/mod.rs `part_ids` (bodies read once per message); src/pimdir/attachment/list.rs: `part` on each file, in text where the section was, the command doc pointing at `attachment download`.
- [x] tests/pimdir_attachments.rs: `a_file_s_part_is_the_one_attachment_download_takes` (each file downloads by its `part`, the same id `attachment list` prints, never its section) and `a_file_whose_body_is_gone_lists_no_part`; the listing test checks `part`.
- [x] Fold into cairn/spec/backends.md and cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
