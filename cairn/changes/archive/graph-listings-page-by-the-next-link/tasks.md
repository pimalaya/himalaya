---
cairn: tasks
change: graph-listings-page-by-the-next-link
---

# Tasks

- [x] src/msgraph/messages/list.rs: `resume` (the page behind a next link, through `page_get`) and `next_page` (the link as Graph gives it, `null` when it leads back) replace `page_skip` and `next_page_token`; the `Next link:` text line removed; test `a_listing_resumes_from_the_next_link_it_printed` over the scripted Graph server, a `$search` link and a foreign one included.
- [x] src/msgraph/mail_folders/list.rs and child_folders.rs: the same.
- [x] src/msgraph/attachments/list.rs: every next link followed.
- [x] Check the Gmail and JMAP listings: conformant, unchanged.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
