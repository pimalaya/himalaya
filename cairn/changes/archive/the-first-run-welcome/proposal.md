---
cairn: change
id: the-first-run-welcome
status: landed
created: 2026-11-11
---

# The first-run welcome

## Why

The wizard left Himalaya on 2026-11-11 (the-wizard-leaves), and a person running it without a configuration now meets a one-line error. Ortie meets the same person with the shared welcome of pimalaya-cli (`welcome::Welcome`): the missing path, the command creating the file from the sample configuration, then the first command to run. pimalaya-config now resolves accounts with typed errors (`ConfigMissingError`, `AccountError`), which is what the welcome keys on, so Himalaya moves onto it and drops the resolution it duplicated.

Himalaya's own `-c` (`ConfigPathsArg`) reads `HIMALAYA_CONFIG` and splits a value on the platform delimiter, the one family member doing either; the shared `ConfigFlags` takes one path per flag and reads no variable.

`account list` never names `pimdir` under BACKENDS, its row builder skipping the block.

## What

- Patch pimalaya-config to ../config. `impl TomlConfig for Config` gains `sample_url` (the config.sample.toml page) and `accounts`; Himalaya's `take_default_account` and its several-defaults `take_account` override go, the shared `take_account` refusing several defaults with the same wording and listing the held accounts when the named one is missing. `NO_CONFIG_HINT` goes.
- `cli::resolve_account` is `Config::load(paths)?` then `take_account(name)?`. `account check` and `account capability` resolve through it, `account list` loads through `Config::load`, so all three name the missing path.
- src/main.rs downcasts `ConfigMissingError`: with `--json` off and stderr a terminal (`welcome::is_wanted`), it prints the welcome on stderr (bin `himalaya`, the sample shipped beside the binary or downloaded from the raw config.sample.toml, then `himalaya envelope list`) and exits 1; otherwise the error goes through the normal report. A bare `himalaya` prints the help when a configuration exists (a broken one included) and meets the missing configuration otherwise.
- `repl` meets the welcome the same way. `mcp` prints its failures on stderr through `pimalaya_cli::mcp::exit_on_failure`, stdout carrying protocol messages only, a missing configuration at a terminal still meeting the welcome.
- **BREAKING**: `-c` becomes pimalaya-cli's `ConfigFlags`: one path per flag, never split, and `HIMALAYA_CONFIG` is no longer read (`-c "$HIMALAYA_CONFIG"` instead). clap's `env` feature goes.
- `account list` names `pimdir` under BACKENDS, in text and JSON.
- README, config.sample.toml header (which documents the delimiter and "the first valid path" although the paths merge), CHANGELOG.

## Scope / non-goals

The REPL and MCP internals stay as they are (the next change moves them onto pimalaya-cli's shared REPL and MCP core). Packaging the sample as share/doc/himalaya/config.sample.toml, which the welcome would copy rather than download, is left out: no release archive or Nix output of Himalaya ships it yet.
