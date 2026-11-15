---
cairn: change
id: pimdir-search-pages-by-token
status: landed
created: 2026-11-15
---

# pimdir search pages by token

## Why

Task H6 of the family's cli-alignment-2026-11-15 plan (finding F12, owner ruling D9: "yes"; ledger C20 for the errors). pimalaya-cli's cairn/spec/output.md ("Paging resumes") asks every listing that can stop early, a ranked search named among them, for `nextPage` taken back by `--page-token`, never a `more` flag. `pimdir search` printed `more` and took `--page` alone. It also failed whole when the store could not read one hit's message, where every other listing lists the rest and names that item in `errors`.

## What

`pimdir search` and `pimdir search --threads` print `nextPage` (the next page number, a string, `null` once the answer is complete) where they printed `more`, and take it back through `--page-token`, pimalaya-cli's shared flag, winning over `--page`; `--page` and `--page-size` stay. A hit whose message the store cannot read is an `errors` entry (pimalaya-cli `ItemError`) with its id, coded `message-unreadable` unless io-pimdir names its own code, the other hits listed and the exit status 3. Breaking for a development build of v3 only, the command being new in it: CHANGELOG and MIGRATION.md say so.
