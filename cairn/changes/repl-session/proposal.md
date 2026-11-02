---
cairn: change
id: repl-session
status: landed
created: 2026-11-02
---

# `himalaya repl`: one session, many commands

## Why

Every himalaya invocation builds its clients from scratch: TCP, TLS, SASL, a secret command (a `pass` decrypt, a keyring prompt, an `ortie token show`), then one command, then teardown. A script listing, reading and flagging ten messages pays that ten times. The only answer today is sirup, which the README (README.md, "Each invocation opens a fresh TCP+TLS+SASL session") points to, and the stateful IMAP commands (`imap select`, `unselect`, `close`, `--no-select`) are documented as useful "over a stateful IMAP session such as a Sirup proxy" because nothing else outlives a command.

The 2027 NLnet plan's Task 1 is "Pimalaya CLI: REPL and MCP". The machine contract it needs is in place since 2026-10-27 (pimalaya-cli cairn/log/2026-10-27-machine-output-contract.md, folded here as "The JSON contract is described and versioned"): coded failures, object roots, newline-delimited `--json`. What is missing is a process that stays up. ortie has one (ortie cairn/spec/repl.md); himalaya does not. The MCP server (cairn/changes/mcp-server) is the second consumer of the same session layer, so this change lands first.

## Overlap with sirup

Both keep a mail session alive across commands, so the question is where each stops. They sit at different layers:

| | sirup | himalaya REPL |
|---|---|---|
| What persists | one authenticated protocol stream per protocol | the typed clients of one account (IMAP, SMTP, JMAP, Graph, Gmail, a local store) |
| Across | processes: any client attaching to the socket | lines of one process's stdin |
| Speaks | raw IMAP, SMTP, ManageSieve bytes | himalaya's command grammar and its JSON contract |
| Credentials | held by the daemon; clients never see them | resolved by himalaya itself |
| Keepalive | NOOP every 4 min while no client is attached | none of its own (see below) |
| Clients | one at a time per socket; the next waits in the listen backlog | the one process that spawned it |
| Its `repl` | a raw reference client (`C:` / `S:` lines, tags generated) for testing the socket | a command session |

Where they overlap, and what to do:

1. **Amortising the handshake.** For one interactive or scripted himalaya session, the REPL makes sirup unnecessary. sirup stays the answer for many short processes (a shell script calling `himalaya` N times, himalaya and himalaya-tui together, non-Pimalaya clients) and for keeping credentials out of the client process. Neither replaces the other; the README says which to pick.
2. **Two commands named `repl`.** `sirup repl imap` relays raw protocol lines; `himalaya repl` runs himalaya commands. Do not merge them: sirup's is a test aid for its own socket and cannot speak typed outputs, and a raw mode in himalaya would duplicate sirup's per-protocol framing (IMAP tags, SMTP continuation lines, ManageSieve literals). The himalaya REPL gets no raw-protocol mode; `imap raw`, `smtp raw` and `sieve raw` stay ordinary commands inside it, caller-tagged as today.
3. **A REPL attached to a sirup socket (set aside).** A session holding a `unix://` connection starves sirup twice: sirup serves one client at a time, so every other client hangs in the backlog until the REPL exits, and sirup only NOOPs while no client is attached, so an idle REPL lets the upstream time out (SMTP's 5 min receiver timeout, RFC 5321 §4.5.3.2.7) and sirup's next keepalive then ends the whole daemon. A fix (connect per command over `unix://`, or sirup refusing a queued client and keeping an attached idle one alive) is left for later; until then the README says not to point a REPL at a sirup socket.
4. **Keepalive.** Not ported from sirup. The REPL is single-threaded and blocked on stdin; a background NOOP would need a thread sharing a client whose stream (and TLS state) cannot be driven from two threads.
5. **Secrets.** sirup reads a shared `pass` entry once for the whole account. The REPL resolves secrets when it builds a client and does not cache them beyond it: a rebuilt client re-resolves, because a secret command often yields a short-lived OAuth token (ortie) that a cache would serve stale.

## What

The CLI reused as is, the way ortie's REPL does: no new interface, only a process that stays alive and keeps its connections.

1. `himalaya repl` starts a session bound to one account (`-a` or the default) and one `--backend`, with `--json` fixed for the session, as ortie's is.
2. Each stdin line is parsed with the one-shot clap grammar and dispatched to the same `execute` methods. `quit`, `exit` and EOF end the session; `repl` (a session inside a session) and `configure` (whose prompts read the terminal the session reads) are refused inside it. A parse or command error is printed and the loop continues; piped, failures go to stdout as `himalaya-error` objects (ortie cairn/spec/repl.md).
3. Clients are connected once and kept. Each is built the first time a command needs it rather than all at startup: an account declaring IMAP, SMTP and ManageSieve would otherwise handshake with all three for a session that only lists envelopes. For a session of shared commands this is the same as connecting at startup.
4. A command that fails drops the client it ran on, so the next command connects again instead of the whole session failing until restart: a server drops an idle session (30 min for IMAP per RFC 3501 §5.4), and a failure mid-exchange may leave the stream out of step. Telling a dead connection from a refused command would mean reading each library's error type (io-imap and io-smtp wrap `io::Error` transparently, so the chain does not expose it), so every failure counts; the cost is one reconnect after a refused command. Nothing is retried.
5. The shared send over SMTP connects for each send and closes after it. A kept SMTP connection dies after 5 idle minutes (RFC 5321 §4.5.3.2.7), so a session's first send after a pause would fail every time; a one-shot run sends once anyway.

What reusing the CLI does not give for free, and why each stays:

- **Lines are split like a shell**, quotes included: ortie's whitespace split cannot carry `envelope list subject "foo bar"`.
- **Dispatch takes a kept client.** Each arm of `Command::execute` (src/cli.rs) used to resolve the account and build its client before calling the command's `execute`. Every arm now runs over a `Session` (src/session.rs) holding one optional client per kind; a one-shot run uses a fresh session for its one command, the REPL keeps it. The command modules do not change.
- **stdin input is refused in the session.** `RawCommandArg` (src/shared/raw.rs), the message argument (src/shared/message/arg.rs), sieve scripts, `message parse -` and `jmap query -` read stdin to EOF; in a piped session stdin is the command stream, so one `message send` with no argument would swallow every following line. Inside the session they take a positional or a file path, and stdin is the coded error `stdin-taken`. The composers' body, which falls back to piped stdin, is empty instead, as at a terminal.

The loop (splitting, TTY detection, quit, printing a failure and continuing) is the second copy after ortie's. It lives in src/repl.rs for now: moving it into pimalaya-cli needs a release there and ortie adopting it, a change of its own.

## Not in scope

Line editing and history (`rlwrap himalaya repl` works). Switching account or backend inside a session. A keepalive, an idle probe, retries. Push or IDLE notifications between commands. Exactly one `--json` value per command, which the MCP server needs (cairn/changes/mcp-server). A REPL over a sirup socket (overlap 3).
