---
cairn: tasks
change: protocol-listings-print-the-raw-payload
---

# Tasks

- [x] X9a src/msgraph/messages/list.rs, src/msgraph/mail_folders/list.rs, child_folders.rs: `conflicts_with = "page_token"`; test `a_query_flag_beside_a_page_token_is_refused`.
- [x] Gmail src/gmail/client.rs `raw_list`, src/gmail/format.rs `str_of`, src/gmail/messages/list.rs, threads/list.rs, drafts/list.rs, history/list.rs; tests `a_page_prints_as_gmail_sent_it` (messages, history).
- [x] Graph src/msgraph/messages/list.rs `raw_page`, mail_folders/list.rs, child_folders.rs, attachments/list.rs; test `a_listing_resumes_from_the_next_link_it_printed` keeps a property io-msgraph does not model.
- [x] JMAP src/jmap/raw.rs, email/query.rs, email/get.rs, mailbox/query.rs, mailbox/get.rs, submission/query.rs, submission/get.rs, submission/create.rs; tests `a_query_keeps_what_the_server_sent`, `a_method_error_fails_the_listing`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
