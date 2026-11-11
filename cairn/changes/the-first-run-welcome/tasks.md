---
cairn: tasks
change: the-first-run-welcome
---

# Tasks

- [x] Cargo.toml: patch pimalaya-config to ../config; pimalaya-cli gains `mcp`; clap loses `env`.
- [x] src/config.rs: `sample_url`, `accounts`; drop `take_default_account`, the `take_account` override and `NO_CONFIG_HINT`.
- [x] src/cli.rs: `resolve_account` on `Config::load` and the shared `take_account`; `ConfigFlags` replaces `ConfigPathsArg`; `meet_bare_invocation`, `welcome`, `SAMPLE_RAW_URL`; the MCP branch on `exit_on_failure`.
- [x] src/main.rs: the welcome on a missing configuration at a terminal; bare invocation through `meet_bare_invocation`.
- [x] src/account/check.rs, capability.rs through `resolve_account`; list.rs through `Config::load`, naming `pimdir`.
- [x] Tests: typed missing configuration, unknown account listing the held ones, several defaults, `account list` naming the missing path and `pimdir`, bare invocation with and without a configuration, `-c` never split.
- [x] README, config.sample.toml header, CHANGELOG, src/main.rs header.
- [x] Manual check under a pseudo-terminal; JSON schemas unchanged.
- [x] Fold the delta into the spec (config, commands, mcp), log, set status landed.
