---
cairn: log
change: gmail-and-graph-read-one-past-the-page
landed: 2026-11-15
---

# Gmail and Graph read one message past the page

Task H20 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15: Gmail and Graph pointed at an empty page after an exactly full last page, their token or next link existing there too.

## What landed

- src/gmail/backend.rs: `page_ids` reads the page's ids through `list_ids`, which asks `messages.list` for the ids up to one past the page (`maxResults` what is left, 500 at most) and follows the token only while short of it, a page bringing no id ending the walk; `more` is whether that id came. `list_envelopes` and `search_envelopes` share it; an unpaged listing keeps reading Gmail's first page.
- src/msgraph/backend.rs: `list_envelopes` asks `$top` one longer (1000 at most, the next link answering past it); `search_envelopes` asks the pages before the page and one message past it in its first request, following the links only while short.
- src/shared/http_script.rs and src/gmail/client.rs `GmailClient::over`: Gmail answers scripted as Graph's, the batch answers multipart.
- Tests: Gmail `an_exactly_full_last_page_has_no_next` (two ids and a token for `maxResults=3`, the token's page empty: `more` false), `a_later_page_is_read_in_one_call` (`maxResults=5` for page 2 of 2), `a_listing_past_the_largest_page_goes_on_by_token`; Graph `an_exactly_full_last_page_has_no_next` (`$top=6` over five messages and a next link), `a_search_page_reads_one_message_past_it`. The old code asked the page size and read the token or link, failing each.
- Not run against Gmail or Graph: live tests are the live testing session's.

## Capabilities moved

- [commands](../spec/commands.md): *Listings resume from their cursor*, Gmail and Graph telling a following page by one message past it.
