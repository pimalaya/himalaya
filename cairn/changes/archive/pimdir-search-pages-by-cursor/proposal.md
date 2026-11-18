---
cairn: change
id: pimdir-search-pages-by-cursor
status: landed
created: 2026-11-18
---

# pimdir search pages by cursor

## Why

Findings TRI-9 and PIM-1 of the family's cli-family-analysis-2026-11-15 plan, Himalaya's part. `pimdir search` paged by offset: page N read N times the page size, a hit arriving or leaving between two pages repeated or hid another, and its token was a page number, so a token of another query was accepted. io-pimdir 0.16.0 turns search paging into a cursor (`PimdirSearchOptions::after`, `PimdirSearchCursor`), and fixes a hostile `VTIMEZONE` aborting the process (PIM-1).

## What

- io-pimdir 0.16.0 from crates.io.
- `nextPage` is a token: a fingerprint of the account, the query, the order and `--threads`, then the cursor's own token; `--page-token` takes it back and refuses one printed under another place, coded `page-token-invalid`.
- `--page` goes, a cursor reaching no page by number. Breaking for a development build of v3 only, the command being new in v3.
