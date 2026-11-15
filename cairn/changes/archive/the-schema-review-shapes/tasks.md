---
cairn: tasks
change: the-schema-review-shapes
---

# Tasks

- [x] A7 src/email/vacation.rs, src/shared/vacation/get.rs, the JMAP and Gmail backends; test `the_window_prints_as_start_and_end`.
- [x] A8 src/shared/message/handler.rs; tests `template_decodes_headers_and_body_and_quotes_names`, `a_template_without_a_sender_prints_null`, tests/message_reply.rs.
- [x] A9 src/email/envelope.rs and every backend building an envelope, src/imap/envelope/thread.rs, src/pimdir/index.rs; test `a_missing_subject_prints_null`.
- [x] A10 src/pimdir/attachment/list.rs; tests/pimdir_attachments.rs.
- [x] B4 src/shared/message/add.rs; test `a_queued_add_says_it_is_queued`.
- [x] B8 src/email/flag.rs (`Flag::spelled`, documented fields), src/maildir/flag/list.rs, src/mbox/flag/list.rs, src/pimdir/flag/list.rs, src/pimdir/index.rs, src/pimdir/tag.rs; tests `a_letter_prints_as_the_shared_flag`, `a_letter_prints_as_the_shared_flag_with_its_header`, `flags_serialize_as_a_list_and_unread_ones_as_null`, `a_tagged_placement_prints_shared_flags`.
- [x] Fold into cairn/spec/commands.md and backends.md; CHANGELOG; MIGRATION.md; the log entry.
