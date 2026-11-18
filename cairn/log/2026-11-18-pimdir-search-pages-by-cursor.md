---
cairn: log
change: pimdir-search-pages-by-cursor
landed: 2026-11-18
---

# pimdir search pages by cursor

Findings TRI-9 and PIM-1 of the family's cli-family-analysis-2026-11-15 plan, Himalaya's part.

## What landed

- Cargo.toml, Cargo.lock: io-pimdir 0.15.1 to 0.16.0 (cursor paging, PIM-1's `VTIMEZONE` abort fixed, `EXTRACTOR` 2 re-extracting an index once).
- src/pimdir/index.rs, src/pimdir/search.rs: the cursor, its token bound by an FNV-1a fingerprint of the place; `--page` removed.
- Tests `a_page_token_is_bound_to_its_query`, `a_token_of_another_query_is_refused` (failing on 002bf012), the paging tests moved to tokens.
- CHANGELOG (Changed, breaking), MIGRATION (development build).

## Capabilities moved

- [search](../spec/search.md): *pimdir searches through the store's index*, its paging and a scenario.
