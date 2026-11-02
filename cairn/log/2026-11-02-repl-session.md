---
cairn: log
change: repl-session
landed: 2026-11-02
---

# `himalaya repl`: one session, many commands

`himalaya repl` runs the CLI's own commands, one per stdin line, over clients kept for the session, so a client connects once rather than once per command. It is Task 1's first half in the 2027 NLnet plan ("Pimalaya CLI: REPL and MCP"), unreleased.

Every command now runs over a `Session` (src/session.rs): one optional client per kind (the shared `EmailClient` and each protocol client), built by `SessionAccount::run` on first use and dropped when a command fails on it. A one-shot run uses a fresh session for its one command, so its behaviour is unchanged; the REPL (src/repl.rs) keeps it. The loop follows ortie's: lines split with `shlex` (quotes, backslashes), `quit`/`exit`/EOF, `repl` and `configure` refused, a parse or command failure printed through the printer and the loop continued, a prompt on stderr at a terminal.

Every failure drops its client, not only a dead connection: io-imap and io-smtp wrap `io::Error` transparently, so the error chain cannot tell a closed socket from a refused command without reading each library's error type, and a failure mid-exchange may leave the stream out of step anyway. The cost is one reconnect after a refused command.

The shared SMTP send now connects for each send and closes after it (`EmailClient::smtp` holds the config, the `SmtpTransport` enum is gone): a kept SMTP connection dies after 5 idle minutes, so a session's first send after a pause would have failed.

Stdin carries the session's commands, so the commands that read their input there (`RawCommandArg`, the message argument, sieve scripts, `message parse -`, `jmap query -`) are refused inside a session with the new code `stdin-taken`, and the composers' body falls back to empty as at a terminal. tests/repl.rs's stdin case fails with the guard disabled: `message add` reads the rest of the session and only one answer comes back instead of three.

Kept out: the loop's move into pimalaya-cli (needs a release there and ortie adopting it), a keepalive or idle probe, and a REPL over a sirup socket (sirup serves one client at a time and keeps the upstream alive only while none is attached; the README warns).

Verified: 368 tests pass with all features; clippy clean on all features; the default, `--no-default-features`, maildir-only, imap+smtp, jmap and sieve builds compile (the smtp-only build fails on io-smtp's `client` feature, already so before this change).

Spec: repl (new: A session runs the CLI over one account, A session connects once, A session never reads command input from stdin, A piped session answers once per line); commands (Shared commands over a selected backend modified, `stdin-taken` added to the code table, repl among the meta commands).
