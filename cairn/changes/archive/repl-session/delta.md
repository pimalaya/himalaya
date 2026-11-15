---
cairn: delta
change: repl-session
---

## ADDED Requirements

### Requirement: A session runs the CLI over one account
`himalaya repl` SHALL start a session bound to one account and one `--backend`, read one command per stdin line, split as a shell splits it, parse it with the one-shot grammar and run it, and loop until `quit`, `exit` or EOF. `repl` and `configure` SHALL be refused inside it. A parse error or a command error SHALL be printed, on stdout as the failure object when piped, and the loop SHALL continue.

### Requirement: A session connects once
Every command SHALL run over a session holding one optional client per kind: a client SHALL be built the first time a command needs it and kept for the commands after it. A command that fails SHALL drop the client it ran on, so the next command builds it again; the failed command SHALL NOT be retried. A one-shot run SHALL use a fresh session for its single command.

### Requirement: A session never reads command input from stdin
A command run inside the session that would read its input from stdin SHALL be refused with the code `stdin-taken`, its input given as a positional or a file path instead. A composer's body given nowhere SHALL be empty, as at a terminal.

### Requirement: A piped session answers once per line
When stdin is not a terminal, every line SHALL produce one flushed answer on stdout: the result, or the failure object (`himalaya-error` under `--json`). When stdin is a terminal, the prompt SHALL render on stderr.

## MODIFIED Requirements

### Requirement: Shared commands over a selected backend
The shared commands SHALL run over an `EmailClient` that owns one backend-client variant per compiled-in backend. It selects the first configured storage backend the global `--backend` flag allows, preferring local backends over network ones, plus an optional SMTP transport for storage backends that cannot send (IMAP, Maildir, m2dir, mbox). The SMTP transport SHALL connect for each send and close after it. Each shared method matches the active backend and calls its per-protocol adapter.
