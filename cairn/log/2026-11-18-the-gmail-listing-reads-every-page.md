---
cairn: log
change: the-gmail-listing-reads-every-page
landed: 2026-11-18
---

# The Gmail listing reads every page

Finding HIM-3 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/gmail/backend.rs: `page_ids` lists an unpaged mailbox through `list_ids` up to its end, `more` false.
- Test `a_listing_without_page_size_reads_every_page`, failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *An unpaged Gmail listing reads every page* (new).
