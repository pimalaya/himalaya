---
cairn: tasks
change: the-gmail-listing-reads-every-page
---

# Tasks

- [x] Scripted test `a_listing_without_page_size_reads_every_page` (src/gmail/backend.rs), failing on the old code (one `messages.list`, the second listing read as the batch answer).
- [x] The unpaged branch of `page_ids` goes through `list_ids`.
- [x] Fold into cairn/spec/backends.md; CHANGELOG; the log entry.
