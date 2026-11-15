---
cairn: tasks
change: unknown-keys-are-refused
---

# Tasks

- [x] Tests failing on the old code: an unknown top-level key, an unknown account key, the `tui` tables loaded beside a refused `tuix`, the v2 `table`, `attachment` and `account` blocks and `envelope.list.table` refused naming MIGRATION.md, the rest of a v1 `[message]` table refused.
- [x] `deny_unknown_fields` on `Config`, `AccountConfig` and the three `message` structs; `tui` at both levels; the v2 blocks refused by name. No `flatten` among the config structs.
- [x] config.sample.toml loads (`the_sample_configuration_parses_with_each_variant`).
- [x] CHANGELOG **BREAKING**, MIGRATION.md v2 to v3.
- [x] Fold the delta into config.md, write the log entry, set `landed`.
- [ ] himalaya-tui moves its keys under `[tui]` and `[accounts.<name>.tui]` (owner).
