---
cairn: delta
change: the-first-run-welcome
---

# Delta

## ADDED Requirements

### Requirement: A missing configuration meets a person with the welcome
(config) When no configuration exists at the first `-c` path or the default location, and stderr is a terminal with `--json` off, a bare `himalaya`, every command resolving an account (`repl` and `mcp` included) and `account list` SHALL print the welcome on stderr and exit 1, printing nothing on stdout. The welcome SHALL frame Himalaya in one sentence, name the missing path, give the commands creating it from config.sample.toml, invite the reader to edit it and uncomment the options they need, and give the `envelope list` command carrying the `-c` paths. When share/doc/himalaya/config.sample.toml exists under the prefix holding the binary's directory or under that directory itself, symlinks resolved, the commands SHALL copy it there with a writable mode, after creating the missing directory; otherwise they SHALL download the raw config.sample.toml with `curl --create-dirs`. Every path SHALL be quoted for the platform's shells, so each command runs as printed. Otherwise the missing-configuration error SHALL stand, as the failure object under `--json`.

#### Scenario: A first run
- GIVEN no file at $XDG_CONFIG_HOME/himalaya/config.toml, $HOME/.config/himalaya/config.toml or $HOME/.himalayarc
- WHEN `himalaya envelope list` runs in a terminal
- THEN the welcome names $XDG_CONFIG_HOME/himalaya/config.toml, the `curl` command downloading config.sample.toml there and `himalaya envelope list`, and the exit status is 1

#### Scenario: A script
- GIVEN no configuration
- WHEN `himalaya --json envelope list` runs
- THEN stdout carries the failure object naming the path looked at and the sample, stderr carries no welcome, and the exit status is 1

### Requirement: A bare invocation shows the help
(config) A bare `himalaya` SHALL print the help when a configuration exists, a broken one included, its error left to the first command reading it. Without one it SHALL fail as a command resolving an account does, the welcome or the error. Nothing prompts.

### Requirement: An account list names every block
(commands) `account list` SHALL print one row per account, `{name, default, backends}`, sorted by name, `backends` naming each block the account declares among `imap`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`, `mbox`, `pimdir`, `smtp` and `sieve`, in that order, whichever backends the build compiles. It SHALL load the configuration without taking an account.

#### Scenario: A pimdir store
- GIVEN an account declaring a `pimdir` block and an `smtp` block
- WHEN `account list` runs
- THEN its BACKENDS cell reads `pimdir,smtp`

## MODIFIED Requirements

### Requirement: Config loading and merge
(config) Without `-c/--config`, the config SHALL load every existing canonical path as a layer, in this order: `$XDG_CONFIG_HOME/himalaya/config.toml`, `$HOME/.config/himalaya/config.toml`, `$HOME/.himalayarc`. Each layer SHALL be deep-merged on top of the ones before, so `$HOME/.himalayarc` overrides. Paths naming the same file SHALL be read once.

`-c/--config` SHALL replace the canonical paths entirely. It MAY be repeated: the first path is the base and the next ones are deep-merged on top, in order. Each occurrence SHALL be one path, never split on a delimiter, and no environment variable SHALL be read for it.

#### Scenario: A Windows path is read whole
- GIVEN `-c C:\Users\me\config.toml`
- WHEN the configuration is loaded
- THEN it is read from that one path, not from `C` and `\Users\me\config.toml`

### Requirement: Missing account is an error
(config) When the config exists but lacks the requested account, the command SHALL fail naming it and listing the accounts the configuration holds, sorted. When no account is named and none is marked default, it SHALL fail naming both ways of picking one. When no configuration is found at all, a command needing one SHALL fail naming the path it looked at (the first `-c` path, else the default location) and pointing at the documented sample, unless a person meets the welcome instead. Himalaya SHALL NOT prompt to create a configuration.

### Requirement: Output discipline
(commands) Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs, the welcome a person meets without a configuration (cairn/spec/config.md), and the failures of `mcp`, whose stdout carries protocol messages only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.

### Requirement: An MCP server over stdio
(mcp) `himalaya mcp` SHALL run an MCP server (revision 2025-06-18) over stdio for one account and one `--backend`, answering `initialize`, `ping`, `tools/list` and `tools/call`, one JSON-RPC message per line, with stdout carrying protocol messages only. Its commands SHALL run over a session kept for the server's lifetime, as the REPL's do, and SHALL never read their input from stdin. A failure of the server itself, resolving its account included, SHALL print as text on stderr and exit 1, a person missing a configuration meeting the welcome instead (cairn/spec/config.md).

## REMOVED Requirements

None.
