---
cairn: tasks
change: the-smaller-live-findings
---

# Tasks

- [x] src/pimdir/performer.rs: `PossibleValuesParser` over the four intents; test `an_unknown_capability_is_refused_naming_the_known_ones`.
- [x] src/pimdir/backend.rs `PimdirFlagged::partial`, src/pimdir/flag/mod.rs `warnings` on `PimdirFlagStageOutput`; test `a_keyword_kept_in_part_is_queued_with_its_warning` (JSON and text).
- [x] src/shared/client.rs `logins`, src/account/context.rs `Account::logins`, src/shared/message/builder.rs `SourceArgs::own` and the reply's `From`, src/shared/message/reply.rs; tests `reply_all_leaves_out_the_reply_s_from_and_the_logins`, `the_logins_are_the_sasl_names_holding_an_address`.
- [x] src/jmap/backend.rs `get_message` scoped by `mailboxIds`; test `a_read_of_an_email_another_mailbox_holds_is_not_found` over the scripted JMAP server.
- [x] pimdir `message read` of an unknown id: already `message-not-found` (H3, 1a8bb275), its test `a_read_of_an_unknown_id_is_not_found`.
- [x] Fold into cairn/spec/backends.md and cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
