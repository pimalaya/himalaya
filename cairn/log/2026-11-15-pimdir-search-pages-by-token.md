---
cairn: log
change: pimdir-search-pages-by-token
landed: 2026-11-15
---

# pimdir search pages by token

Task H6 of the family's cli-alignment-2026-11-15 plan (finding F12, owner ruling D9; ledger C20).

## What landed

- src/pimdir/search.rs: `pimdir search` takes pimalaya-cli's `PageTokenArg` (`--page-token`, a page number from 1, winning over `--page`) and prints `{hits, nextPage, coverage, errors}`, `{threads, nextPage, coverage, errors}` with `--threads`, where it printed `more`. Text prints `Next page:` with `--page` as before, then the entries after the items. A non-empty `errors` exits 3.
- src/pimdir/index.rs: `mail_hits` lists the hits `mail_hit` reads and makes each one it cannot read an item error, its id the hit's `seq`, coded `message-unreadable` unless io-pimdir names its own code; a thread keeps its other hits. `pimdir thread` still fails whole.
- Tests: `a_hit_the_store_cannot_read_is_an_item_error` corrupts one `mail_summary` row (CHECKs ignored) and fails on the old code, where the whole search failed with "Invalid column type"; `a_page_names_the_next_and_its_errors` checks the keys and the text; tests/pimdir_search.rs runs the binary over a filled store for `nextPage`, `--page-token`, `--threads`, a refused token and the item error with exit 3.
- CHANGELOG (**BREAKING**) and MIGRATION.md, under "From a development build of v3", the command being new in v3.

The shape for Cardamum and Calendula (C9, C20, K9): `nextPage` a string page number or `null`, `--page-token` from pimalaya-cli's `PageTokenArg`, `errors` a list of pimalaya-cli `ItemError` `{id, code, error, sources}` after `coverage`, exit 3 when not empty.

## Capabilities moved

- [search](../spec/search.md): *pimdir searches through the store's index*, its output, paging and item errors, two scenarios.
- [commands](../spec/commands.md): *A command acting on several items lists what failed* names `pimdir search`; the `message-unreadable` code row.
