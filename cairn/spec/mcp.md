---
cairn: spec
capability: mcp
status: current
---

# MCP server

`himalaya mcp` serves the shared API to Model Context Protocol clients over stdio: each tool is a shared command, its input schema generated from the command's grammar and its result the command's `--json` payload, run over clients kept for the server's lifetime (cairn/spec/repl.md). It lists the tools that read by default, and no tool reads or writes a local file.

### Requirement: An MCP server over stdio
`himalaya mcp` SHALL run an MCP server (revision 2025-06-18) over stdio for one account and one `--backend`, answering `initialize`, `ping`, `tools/list` and `tools/call`, one JSON-RPC message per line, with stdout carrying protocol messages only. Its commands SHALL run over a session kept for the server's lifetime, as the REPL's do, and SHALL never read their input from stdin.

### Requirement: Tools are the classified shared API
The server SHALL expose one tool per leaf command of the shared API that is classified as reading, writing or sending and that the account's transports serve at least one capability of, as the static tier knows without connecting, named `<group>_<leaf>`, and no protocol-specific or raw passthrough command. A tool whose capability only the server decides SHALL stay listed and answer `capability-missing` as a tool error when the server lacks it.

#### Scenario: A send-only account
- WHEN `himalaya mcp --allow-write --allow-send` serves an account configuring SMTP alone
- THEN `tools/list` holds `message_send` and `message_compose`, and no `envelope_list` or `flag_add`

### Requirement: The search tool says what the backend answers
The `envelope_search` description SHALL end with the clauses and sort keys of the query language the account's storage backend answers fully, in part with each detail, and not at all, from the capability tables.

### Requirement: A tool's input is the command's grammar
A tool's `inputSchema` SHALL be generated from its clap command, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command. An unknown argument SHALL be refused naming the known ones.

### Requirement: A tool's output is the command's payload
A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is an object and the tool has one output shape. A refused or failed call SHALL return `isError: true` with the `himalaya-error` object; an unknown tool SHALL be a JSON-RPC error.

### Requirement: Writing and sending are opt-in
Without flags the server SHALL list reading tools only, without the arguments that write. `--allow-write` SHALL add the tools and arguments that change the store, `mailbox_create`, `mailbox_rename` and `mailbox_delete` among them, and `--allow-send` those that send mail, `vacation_set` among them, an auto-reply once on answering mail.

### Requirement: A tool reads and writes no local file
No tool SHALL take a local path: `attachment download` and `message parse` SHALL NOT be exposed, path arguments SHALL NOT be properties, and a message argument SHALL be taken as the message itself even when it spells an existing path.
