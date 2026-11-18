---
cairn: tasks
change: single-reads-print-raw
---

# Tasks

- [x] Gmail: src/gmail/profile/get.rs, drafts/get.rs, messages/get.rs, threads/get.rs, settings/*/get.rs, the five settings sets through `settings::raw_put`, `GmailClient::raw_get_with`, `FormatArg::query`.
- [x] Graph: src/msgraph/messages/get.rs.
- [x] JMAP: src/jmap/vacation/get.rs through `raw::get`, src/jmap/raw.rs naming the vacation capability.
- [x] Tests: one per read, an unmodelled vendor key surviving (`the_profile_prints_as_gmail_sent_it`, `a_draft_prints_as_gmail_sent_it`, `a_message_prints_as_gmail_sent_it`, `a_thread_prints_as_gmail_sent_it`, `the_settings_print_as_gmail_sent_them`, `a_set_prints_what_gmail_answered`, `an_alias_prints_as_gmail_sent_it`, `a_message_prints_as_graph_sent_it`, `the_vacation_response_prints_as_sent`, `a_vacation_response_get_names_its_capability`).
- [x] Fold into cairn/spec/commands.md, CHANGELOG, MIGRATION.
