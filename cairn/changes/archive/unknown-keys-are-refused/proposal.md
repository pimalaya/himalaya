---
cairn: change
id: unknown-keys-are-refused
status: landed
created: 2026-11-14
---

# Unknown keys are refused

> Cross-repo change. Order: **himalaya** (this side) → **himalaya-tui**, which moves its keys under the `tui` tables.

## Why

The top level, the account blocks and the `message` table tolerated any key, so a typo (`downloads-dri`, `emial`) loaded silently and the setting it meant did nothing. They were open because himalaya-tui shares the file and adds its own keys there (top-level `keybinds`, `contact-command`, `contact-complete-key`, `theme`; account `contact-command`), and so that the rest of a v1 `[message]` table kept loading. The v2 `envelope.list.table` block was ignored for the same reason, though himalaya-tui reads no `table` key; the v2 `table`, `attachment` and `account` blocks were ignored at the open levels while `mailbox.list.table` was refused.

The owner decided on 2026-11-14 that every level refuses an unknown key, and that himalaya-tui's keys move under a `[tui]` table at the top level and `[accounts.<name>.tui]` in an account.

## What

- `#[serde(deny_unknown_fields)]` on `Config`, `AccountConfig`, `MessageConfig`, `MessageQuoteConfig` and `MessageSendConfig`, the only config structs without it. No config struct uses `#[serde(flatten)]`.
- `tui: Option<IgnoredAny>` at the top level and in an account, loaded whatever it holds and never read.
- The v2 blocks are refused by name, ``The `<block>` block is removed since v3: delete it, see MIGRATION.md``: `table` and `attachment` at both levels, the top-level `account`, and `envelope.list.table`. The v1 `backend` table and the `m2dir` block keep loading, named by the no-backend error.
- The `from` and `from-name` aliases himalaya-tui writes stay.
- himalaya-tui is not edited here: the owner moves its keys after.
- Spec: config ("Unknown keys are refused" added; "A v1 account layout is named, not silently ignored", "Sent copies are configured per account" and "No colour is configured" modified). CHANGELOG **BREAKING**, MIGRATION.md.
