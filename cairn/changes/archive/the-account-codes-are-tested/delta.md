---
cairn: delta
change: the-account-codes-are-tested
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A session runs the CLI over one account
`himalaya repl` SHALL start a session bound to one account, its configuration and one `--backend`, and run pimalaya-cli's shared REPL loop over it: read one command per stdin line, split as a shell splits it, a blank line or a line whose first word starts a `#` comment skipped, parse it with the one-shot grammar plus `--json`, subcommand names inferred from a unique prefix, and run it, until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it with `Cannot start a session inside a session`, nothing started. A line whose quote is not closed, a line that is not UTF-8, a parse error or a command error SHALL be printed, on stdout as the failure object when piped, a parse error coded `usage-invalid` with clap's message without its `error: ` prefix, a command error coded as a one-shot run codes it, and the loop SHALL continue.

#### Scenario: A session inside a session
- GIVEN a piped session started with `--json`
- WHEN the lines `repl`, `mcp --allow-write` and `envelope list` are read
- THEN the first two answer the failure object `Cannot start a session inside a session` and the third lists the inbox

## REMOVED Requirements

None.
