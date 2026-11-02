---
cairn: tasks
change: repl-session
---

# Tasks

Task 1 of the 2027 NLnet plan, "Pimalaya CLI: REPL and MCP".

## himalaya (this repository)

- [x] `Session` (src/session.rs): one optional client per kind, built on first use, dropped when a command fails on it; every `Command::execute` arm runs over it, a one-shot run with a fresh one
- [x] `Command::Repl` (src/repl.rs): shell-style splitting (`shlex`), `quit`/`exit`/EOF, `repl` and `configure` refused, failures printed and the loop continued
- [x] Shared SMTP send connects per send
- [x] Refuse stdin input in the session (`RawCommandArg`, message argument, sieve scripts, `message parse -`, `jmap query -`) with `stdin-taken`; composer bodies empty
- [x] Tests: `session::tests` (built once, rebuilt after a failure); tests/repl.rs (one answer per line, failures survived, stdin refused; the stdin case fails with the guard disabled)
- [x] Docs: `imap select`, `unselect`, `close`, `--no-select`; README; main.rs architecture header
- [x] CHANGELOG; fold into a new cairn/spec/repl.md and cairn/spec/commands.md; log; land

## Later

- [ ] pimalaya-cli: the loop made generic (splitting, TTY versus piped, quit, failure printed and loop continued), then himalaya and ortie on it
- [ ] sirup: a client queued behind another gets an immediate refusal instead of hanging; keepalive while an attached client is idle
