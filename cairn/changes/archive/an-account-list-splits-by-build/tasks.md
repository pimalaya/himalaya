---
cairn: tasks
change: an-account-list-splits-by-build
---

# Tasks

- [x] src/account/list.rs on `AccountListOutput` and `AccountRow`, the blocks split by `Backend::COMPILED`.
- [x] src/account/check.rs on `AccountCheckOutput` and `BackendCheck`; src/json_schema.rs registers the kit's types.
- [x] tests/account.rs `account_list_splits_the_declared_backends_by_build`, run on a full build and on `--no-default-features --features rustls-ring,maildir`.
- [x] Fold into cairn/spec/commands.md; CHANGELOG, MIGRATION.md; the log entry.
