---
cairn: change
id: a-removed-table-reads-the-family-sentence
status: landed
created: 2026-11-15
---

# A removed table reads the family sentence

## Why

Task H25 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15 (contract.md, H-7): a v1 `folder.alias.inbox` got serde's generic "unknown field `folder`", naming neither `mailbox.alias` nor MIGRATION.md, though MIGRATION.md says the v1 `[folder.alias]` block became `[mailbox.alias]`. pimalaya-config 0.5.1 gives the family one sentence for a removed key or table (`removed_key!`), which Himalaya's own refusals (``The `table` block is removed since v3: delete it, see MIGRATION.md``) did not use.

## What

Himalaya moves to pimalaya-config 0.5.1 and refuses through `removed_key!`: the v2 `table`, `attachment`, top-level `account` and `envelope.list.table` tables read ``The `<name>` table is removed, the output being plain text: delete it, see MIGRATION.md``, and v1's `folder` table, at the top level and in an account, ``The `folder` table is removed, v2 renaming it `mailbox`: move `folder.alias` to `mailbox.alias` and delete the rest, see MIGRATION.md``. The v1 `backend` table and the `m2dir` block keep loading as their requirements say. serde's list of expected fields in an unknown-key error still names the refused tables (L13, pimalaya-config's).
