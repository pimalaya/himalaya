---
cairn: log
change: unknown-keys-are-refused
landed: 2026-11-14
---

# Unknown keys are refused

Every table of the configuration refuses a key it does not know: `Config`, `AccountConfig`, `MessageConfig`, `MessageQuoteConfig` and `MessageSendConfig` took `deny_unknown_fields`, the other config structs having it already; none uses `flatten`. The rest of a v1 `[message]` table (`read`, `write`, `send.backend`) is refused with them.

himalaya-tui's keys go under `tui`, at the top level and in an account, loaded as `IgnoredAny` and never read. The v2 blocks are refused by name, ``The `<block>` block is removed since v3: delete it, see MIGRATION.md``: `table` and `attachment` at both levels, the top-level `account`, and `envelope.list.table`, which was ignored for himalaya-tui though it reads no `table` key. The v1 `backend` table and the `m2dir` block keep loading, named by the no-backend error. himalaya-tui is not edited: its keys move under `tui` on its side.

Tested: an unknown top-level key, an unknown account key, the `tui` tables at both levels beside a refused `tuix`, the v2 blocks at both levels, `envelope.list.table` at both levels and the v1 `[message]` keys, each failing on the old code. `save_copy_reads_a_v1_boolean` no longer loads v1 keys beside the boolean. config.sample.toml still loads as written and in every variant. Through the binary, `envelope.list.table.unseen-char` fails pointing at its line and MIGRATION.md, and an account's `contact-command` fails as an unknown field.

Spec: config ("Unknown keys are refused" added; "A v1 account layout is named, not silently ignored", "Sent copies are configured per account" and "No colour is configured" modified).
