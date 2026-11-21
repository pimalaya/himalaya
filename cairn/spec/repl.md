---
cairn: spec
capability: repl
status: current
---

# REPL

`himalaya repl` keeps one process alive for one account and runs the CLI's own commands over its stdin, so each client connects once for the session rather than once per command. Every command, one-shot or not, runs over a session holding the clients it built. The loop is pimalaya-cli's shared REPL (pimalaya-cli cairn/spec/repl.md), Himalaya adding `--backend` to the flags a session fixes.

### Requirement: A session runs the CLI over one account
`himalaya repl` SHALL start a session bound to one account, its configuration and one `--backend`, and run pimalaya-cli's shared REPL loop over it: read one command per stdin line, split as a shell splits it, a blank line or a line whose first word starts a `#` comment skipped, parse it with the one-shot grammar plus `--json`, subcommand names inferred from a unique prefix, and run it, until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it by pimalaya-cli, by name, alias or inferred prefix, with `Cannot start a session inside a session` coded `usage-invalid`, nothing started. A line whose quote is not closed, a line that is not UTF-8, a parse error or a command error SHALL be printed, on stdout as the failure object when piped, a parse error coded `usage-invalid` with clap's message without its `error: ` prefix, a command error coded as a one-shot run codes it, and the loop SHALL continue. The session SHALL resolve its account before reading the first line, so a missing configuration or account fails the session rather than its first command.

#### Scenario: A session inside a session
- GIVEN a piped session started with `--json`
- WHEN the lines `repl`, `mcp --allow-write` and `envelope list` are read
- THEN the first two answer the failure object `Cannot start a session inside a session`, coded `usage-invalid`, and the third lists the inbox

### Requirement: A session connects once
Every command SHALL run over a session holding one optional client per kind: a client SHALL be built the first time a command needs it and kept for the commands after it. A command that fails SHALL drop the client it ran on, so the next command builds it again; the failed command SHALL NOT be retried. A one-shot run SHALL use a fresh session for its single command.

### Requirement: A session never reads command input from stdin
A command run inside the session that would read its input from stdin SHALL be refused with the code `stdin-taken`, its input given as a positional or a file path instead. A composer's body given nowhere SHALL be empty, as at a terminal.

#### Scenario: A message given nowhere
- GIVEN a piped session
- WHEN the line `message add --mailbox inbox` is followed by `envelope list`
- THEN the first answers `stdin-taken` and the second lists the inbox

### Requirement: A piped session answers once per line
When stdin is not a terminal, every line SHALL produce one flushed answer on stdout: the result, or the failure object (`himalaya-error` under `--json`). Nothing SHALL be added after a result, every text output ending with its own newline, and a command ending with a partial outcome SHALL print its result alone. When stdin is a terminal, the prompt `himalaya> ` SHALL render on stderr, a line clap cannot parse SHALL print clap's rendering on stderr, and a failed command SHALL print the failure object on stdout.

### Requirement: A line switches itself to JSON
A line carrying `--json` SHALL print its answer as JSON, the lines after it keeping the session's format. `--json` given to `repl` SHALL make every line JSON.

#### Scenario: One JSON line in a text session
- GIVEN a text session
- WHEN the lines `envelope list --json` and `envelope list` are read
- THEN the first prints one JSON value and the second the text list, nothing between them

### Requirement: Session flags are refused by name
A line clap cannot parse that sets a flag the session fixes when it starts, `-a`, `--account`, `-c`, `--config`, `--log-level`, `--log`, `--log-file`, `-b` or `--backend`, SHALL fail with ``Cannot set `<flag>` inside a session, start a new session with it instead`` in place of clap's error, the flag recognised alone, as `--<long>=<value>` or as a short flag with its value attached, before any `--`.

#### Scenario: Another backend on one line
- GIVEN a piped session started with `--json`
- WHEN the line `envelope list -b maildir` is read
- THEN it answers the failure object ``Cannot set `-b` inside a session, start a new session with it instead``

### Requirement: Help keeps JSON lines whole
A line asking for help or the version SHALL print clap's text on stdout in a text session, and on stderr when the session's `--json` or a `--json` word of the line before any `--` is set, so stdout keeps one JSON value per line. The version SHALL be the one the command enum carries, the binary's: `-V` its version, `--version` its long version, after a command too, named after the command as the command line names it (`himalaya-envelope-list v<version> ...`). Its usage line SHALL name the binary before the command, as `Usage: himalaya envelope list [OPTIONS]`. A failing line SHALL hint at restarting the session with `--log-level` for more verbose logs, the flag being fixed for the session.

#### Scenario: The version in a session
- GIVEN a text session
- WHEN the lines `-V` and `envelope list --version` are read
- THEN stdout holds `himalaya <version>`, then the long version the command line prints for `himalaya envelope list --version`

### Requirement: A session can be kept alive
`Session::keep_alive` SHALL send `NOOP` on the unified client's IMAP connection when the session holds one, and SHALL do nothing when the slot is empty or the backend keeps no connection. A keepalive that fails SHALL empty the slot, as a failed command does, so the next command builds the client again. No command of the grammar SHALL call it. `Session::is_connected` SHALL say whether the unified client is built, so a caller knows the next command connects first.

#### Scenario: An idle IMAP connection
- GIVEN a session whose unified client holds an IMAP connection
- WHEN `Session::keep_alive` runs
- THEN one `NOOP` is sent on that connection and the client is kept

#### Scenario: A dead connection
- GIVEN a session whose IMAP connection the server closed
- WHEN `Session::keep_alive` runs
- THEN it fails, the slot is emptied, and the next command connects again
