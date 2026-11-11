---
cairn: tasks
change: the-wizard-leaves
---

# Tasks

- [x] Delete src/wizard/, the `configure` command and its `wizard` alias.
- [x] Cargo.toml: drop the `wizard` feature, its place in `default`, io-pim-discovery and its TLS forwards, the pimalaya-cli wizard features and shellexpand; patch pimalaya-cli to ../cli.
- [x] src/main.rs: bare `himalaya` prints the help; header no longer describes the wizard or discovery.
- [x] src/cli.rs: drop `offer_configuration`, the offer inside `resolve_account` and the long_about pitch.
- [x] src/config.rs: one `NO_CONFIG_HINT` pointing at the sample; drop `CONFIG_SAMPLE_URL`, `AccountConfig::render`, `RENDER_ORDER`, the `Serialize` derives and the `skip_serializing_if` helpers.
- [x] src/json_schema.rs: drop `himalaya-configure`.
- [x] src/repl.rs: drop the guard refusing `configure`.
- [x] src/account/check.rs: drop `test_account` and `probe_imap_mechanisms`, the connectors back to private.
- [x] README, CONTRIBUTING, config.sample.toml.
- [x] CHANGELOG: one Removed entry, the Unreleased wizard entries dropped.
- [x] Move the six landed wizard changes to changes/archive/.
- [x] Fold the delta into the spec (wizard.md removed; config, commands, packaging, provider-quirks, repl), log, set status landed.
