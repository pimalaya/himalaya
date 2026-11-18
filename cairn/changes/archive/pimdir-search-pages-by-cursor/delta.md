---
cairn: delta
change: pimdir-search-pages-by-cursor
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: pimdir searches through the store's index
(search) `--page-size` and `--page-token` page the hits. `nextPage` SHALL be the token of the next page, `null` once the answer is complete: the place of the last hit or thread the page answered (io-pimdir `PimdirSearchCursor`), bound to the account, the query, the order and `--threads`. A token printed under another account, query, order or listing, or one no search printed, SHALL be refused, coded `page-token-invalid`. `pimdir search` SHALL take no page number. A scenario: a token of another query.

## REMOVED Requirements

None.
