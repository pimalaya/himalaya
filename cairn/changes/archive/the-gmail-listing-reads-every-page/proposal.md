---
cairn: change
id: the-gmail-listing-reads-every-page
status: landed
created: 2026-11-18
---

# The Gmail listing reads every page

## Why

Finding HIM-3 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. On Gmail, `--page-size 0` sent one `messages.list` with no `maxResults` (Gmail serves 100) and dropped `nextPageToken`, so `envelope list` and `envelope search` listed the first 100 messages and said the listing was complete. cairn/spec/commands.md says `--page-size 0` lists the whole mailbox; Graph follows every link and IMAP fetches `1:*`. The code's own doc comment stated the opposite of the spec.

## What

- `page_ids` (src/gmail/backend.rs) reads an unpaged listing through `list_ids` as a paged one is, token after token at 500 a call, and says no page follows.
- The contradicting doc sentence is replaced.
