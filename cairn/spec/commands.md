---
cairn: spec
capability: commands
status: current
---

# Commands

The command tree splits into three groups. The shared API (mailbox, envelope, flag, message, attachment) is the cross-protocol least-common-denominator surface, behaving the same whatever backend serves the active account. The protocol-specific APIs (imap, jmap, gmail, msgraph, maildir, m2dir, smtp) each expose the full surface of one backend, including operations the shared API cannot model. The meta commands (account, completion, manual) cover account configuration, shell completions and man pages.

### Requirement: Shared commands over a selected backend
The shared commands SHALL run over an `EmailClient` that owns one backend-client variant per compiled-in backend. It selects the first configured storage backend the global `--backend` flag allows, preferring local backends over network ones, plus an optional SMTP transport for storage backends that cannot send (IMAP, Maildir, m2dir). Each shared method matches the active backend and calls its per-protocol adapter.

### Requirement: Protocol commands ignore backend selection
Each protocol command SHALL build its own `<Proto>Client` via a `build_<proto>_client` helper and run against that backend directly, ignoring `--backend`. The imap command mirrors IMAP's flat command list; gmail and msgraph track their REST resource domains; the filesystem backends expose only operations that map to their on-disk layout, leaving MIME rendering to the shared commands.

### Requirement: Raw passthrough is byte-verbatim
The `imap raw` and `smtp raw` commands SHALL forward the command bytes to the server verbatim, resolving the argument through the shared `RawCommandArg` (positional or stdin). It decodes literal `\r` / `\n` escapes into real CRLF so a shell-typed command survives intact. `imap raw` sends a batch of caller-tagged commands: it appends a trailing CRLF when missing and delegates tagging, framing and out-of-order completion tracking to io-imap. `smtp raw` stays a single command line: it strips the trailing CRLF (io-smtp appends its own) and rejects a multi-line batch, since the SMTP exchange reads exactly one reply.

### Requirement: Account threaded as a sibling argument
The active account context SHALL be threaded as a sibling argument through every `execute` chain, never reached through the client. Subcommands receive `account` and `client` side by side.

### Requirement: Output discipline
Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.
