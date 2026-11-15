---
cairn: log
change: the-page-token-is-long-only
landed: 2026-11-15
---

# The page token is long only

Task H7 of the family's cli-alignment-2026-11-15 plan (finding F26, owner ruling D11).

## What landed

- src/pimdir/attachment/list.rs: `--page-token` of `pimdir attachment list` and `search` (one `PageArgs`) takes no `-t`, which is `--to` across Himalaya. `the_page_token_takes_no_short_letter` checks both commands.
- CHANGELOG (**BREAKING**) and MIGRATION.md, under "From a development build of v3", the commands being new in v3.

## Capabilities moved

- [backends](../spec/backends.md): *pimdir lists the account's attachments*, its paging paragraph.
