---
cairn: tasks
change: gmail-and-graph-read-one-past-the-page
---

# Tasks

- [x] src/gmail/backend.rs: `page_ids`, shared by `list_envelopes` and `search_envelopes`, and `list_ids`, the token walk; src/msgraph/backend.rs: `list_envelopes` asks `$top` one longer, `search_envelopes` reads one message past the page.
- [x] src/shared/http_script.rs, src/gmail/client.rs `GmailClient::over`: a scripted HTTP stream for the Gmail tests, the Graph ones moving onto it.
- [x] Tests on scripted servers: Gmail `an_exactly_full_last_page_has_no_next`, `a_later_page_is_read_in_one_call`, `a_listing_past_the_largest_page_goes_on_by_token`; Graph `an_exactly_full_last_page_has_no_next`, `a_search_page_reads_one_message_past_it`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; the log entry.
