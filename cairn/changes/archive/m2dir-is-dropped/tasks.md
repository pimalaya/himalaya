---
cairn: tasks
change: m2dir-is-dropped
---

# Tasks

- [x] Delete src/m2dir/ and the `m2dir` module declaration in src/main.rs.
- [x] Cargo.toml: drop the `m2dir` feature and io-m2dir; Cargo.lock follows.
- [x] build.rs: drop `M2DIR` from the `backend` cfg.
- [x] src/backend.rs: drop `Backend::M2dir` and `allows_m2dir`.
- [x] src/shared/client.rs, src/session.rs, src/cli.rs: drop the m2dir client, session slot, command and arms.
- [x] src/capability.rs: drop the `M2DIR` table and its place in `declared` and `STORAGE_ORDER`.
- [x] src/account/check.rs, src/account/list.rs: drop the m2dir check and the `m2dir` backend name.
- [x] src/json_schema.rs: drop the m2dir schemas.
- [x] src/config.rs: `m2dir` becomes an ignored block named by `no_backend_error`; `M2dirConfig` goes; tests follow.
- [x] Every `feature = "m2dir"` of the cfg lists, and the comments naming m2dir.
- [x] config.sample.toml, .github/workflows/tests.yml.
- [x] README, CONTRIBUTING, MIGRATION, CHANGELOG.
- [x] Remove cairn/spec/testing/m2dir.md and m2dir-specific.md and their row in cairn/spec/testing/README.md.
- [x] Fold the delta into the spec (backends, commands, config, packaging, search), log, set status landed.
