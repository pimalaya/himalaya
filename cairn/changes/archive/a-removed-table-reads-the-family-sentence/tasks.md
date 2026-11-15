---
cairn: tasks
change: a-removed-table-reads-the-family-sentence
---

# Tasks

- [x] Cargo.toml `pimalaya-config = "0.5.1"`, `cargo update -p pimalaya-config` (the lockfile diff kept to that crate).
- [x] src/config.rs, src/config/envelope.rs: the `removed()` helper and the `refuse_*` deserializers replaced by `removed_key!` types, `folder` added at both levels.
- [x] Tests: `the_v2_blocks_are_refused_naming_migration` and `envelope_table_is_refused_naming_migration` assert the family sentence; `the_v1_folder_table_names_mailbox_alias` at the top level and in an account, failing on the old code ("unknown field `folder`").
- [x] Fold into cairn/spec/config.md; CHANGELOG; MIGRATION.md; the log entry.
