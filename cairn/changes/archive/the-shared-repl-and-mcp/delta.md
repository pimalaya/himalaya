---
cairn: delta
change: the-shared-repl-and-mcp
---

# Delta

## ADDED Requirements

### Requirement: A line switches itself to JSON
(repl) A line carrying `--json` SHALL print its answer as JSON, the lines after it keeping the session's format. `--json` given to `repl` SHALL make every line JSON.

#### Scenario: One JSON line in a text session
- GIVEN a text session
- WHEN the lines `envelope list --json` and `envelope list` are read
- THEN the first prints one JSON value and the second the text list, nothing between them

### Requirement: Session flags are refused by name
(repl) A line clap cannot parse that sets a flag the session fixes when it starts, `-a`, `--account`, `-c`, `--config`, `--log-level`, `--log`, `--log-file`, `-b` or `--backend`, SHALL fail with ``Cannot set `<flag>` inside a session, start a new session with it instead`` in place of clap's error, the flag recognised alone, as `--<long>=<value>` or as a short flag with its value attached, before any `--`.

#### Scenario: Another backend on one line
- GIVEN a piped session started with `--json`
- WHEN the line `envelope list -b maildir` is read
- THEN it answers the failure object ``Cannot set `-b` inside a session, start a new session with it instead``

### Requirement: Help keeps JSON lines whole
(repl) A line asking for help SHALL print clap's text on stdout in a text session, and on stderr when the session's `--json` or a `--json` word of the line before any `--` is set, so stdout keeps one JSON value per line.

### Requirement: The server tells the agent how to use it
(mcp) `initialize` SHALL carry `instructions` saying how to find a mailbox (`mailbox_list`, the `mailbox` argument, the inbox when left out) and an envelope id (`envelope_list`, `envelope_search` and its query language), that `attachment_download` returns the bytes as an embedded resource and the composers take files inline as `attach-data`, that `--allow-write` and `--allow-send` gate the tools and arguments that change the store and send mail, and that a failed call answers `isError` with the `himalaya-error` object. Every tool name it holds SHALL be one of the tool table.

## MODIFIED Requirements

### Requirement: A session runs the CLI over one account
(repl) `himalaya repl` SHALL start a session bound to one account, its configuration and one `--backend`, and run pimalaya-cli's shared REPL loop over it: read one command per stdin line, split as a shell splits it, a blank line or a line whose first word starts a `#` comment skipped, parse it with the one-shot grammar plus `--json`, subcommand names inferred from a unique prefix, and run it, until `quit`, `exit` or EOF. `repl` and `mcp` SHALL be refused inside it with `Cannot start a session inside a session`. A line whose quote is not closed, a line that is not UTF-8, a parse error or a command error SHALL be printed, on stdout as the failure object when piped, and the loop SHALL continue.

### Requirement: A piped session answers once per line
(repl) When stdin is not a terminal, every line SHALL produce one flushed answer on stdout: the result, or the failure object (`himalaya-error` under `--json`). Nothing SHALL be added after a result, every text output ending with its own newline, and a command ending with a partial outcome SHALL print its result alone. When stdin is a terminal, the prompt `himalaya> ` SHALL render on stderr, a line clap cannot parse SHALL print clap's rendering on stderr, and a failed command SHALL print the failure object on stdout.

### Requirement: An MCP server over stdio
(mcp) `himalaya mcp` SHALL run an MCP server (revision 2025-06-18) over stdio for one account and one `--backend` on pimalaya-cli's shared MCP core, answering `initialize`, `ping`, `tools/list` and `tools/call`, one JSON-RPC 2.0 message per line, with stdout carrying protocol messages only. A line that is not UTF-8 or not JSON SHALL answer -32700 and one that is not a JSON object, a batch included, -32600, both with a null `id`; a request whose `jsonrpc` is not `"2.0"` SHALL answer -32600 with its `id`; the server SHALL read on after each. `initialize` SHALL answer the client's `protocolVersion` when it is 2025-06-18, and 2025-06-18 otherwise. Its commands SHALL run over a session kept for the server's lifetime, as the REPL's do, and SHALL never read their input from stdin. A failure of the server itself, resolving its account included, SHALL print as text on stderr and exit 1, a person missing a configuration meeting the welcome instead (cairn/spec/config.md).

#### Scenario: A batch
- GIVEN a server
- WHEN the line `[{"jsonrpc":"2.0","id":1,"method":"ping"}]` is read, then a `ping` with id 4
- THEN the first answers -32600 with `id` `null` and the second `{}`

### Requirement: A tool's input is the command's grammar
(mcp) A tool's `inputSchema` SHALL be generated from its clap command by pimalaya-cli's `mcp::grammar::Grammar`, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command from the argv the grammar writes. The one exception SHALL be the server-only arguments its tool declares, which clap never sees. An argument the `inputSchema` does not declare SHALL be refused before the command runs as a tool error ``Unknown argument `<key>`, <tool> takes <properties>``, the server-only ones named among them. A value of the wrong type, and a count other than one for an argument the tool takes one value of, SHALL be refused the same way.

### Requirement: A tool's output is the command's payload
(mcp) A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is typed `object` and the tool has one output shape, save `attachment_download`, whose bytes travel as an embedded resource instead. The events a command prints SHALL come first in `content`, one text item each, the result last. A command ending with a partial outcome (a refused IMAP, SMTP or ManageSieve completion, a failed `account check`) SHALL keep its result as `structuredContent` and set `isError: true`. A refused or failed call, a command printing no value or several (`The command printed <n> values, one expected`) and a part tool whose command printed no part (`The command printed no part to embed`) included, SHALL return `isError: true` with the `himalaya-error` object and no `structuredContent`; an unknown tool SHALL be a JSON-RPC error.

### Requirement: Output discipline
(commands) Data and errors SHALL go to stdout through the printer; `--json` switches every command to JSON. stderr carries logs, the welcome a person meets without a configuration (cairn/spec/config.md), the REPL's prompt and its help in a JSON session, and the failures of `mcp`, whose stdout carries protocol messages only. Each command's doc comment is its `--help` text, so `himalaya <command> --help` is the canonical per-command usage reference.

## REMOVED Requirements

None.
