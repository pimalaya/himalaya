---
cairn: spec
capability: mcp
status: current
---

# MCP server

`himalaya mcp` serves the unified API to Model Context Protocol clients over stdio: each tool is a unified command, its input schema generated from the command's grammar (plus the server-only arguments its tool declares) and its result the command's `--json` payload, run over clients kept for the server's lifetime (cairn/spec/repl.md). It lists the tools that read by default, no tool reads or writes a local file, and an attachment comes back as an embedded resource, bounded by the server. The protocol answers, the call checks, the input grammar and the tool results are pimalaya-cli's shared MCP core (pimalaya-cli cairn/spec/mcp.md); the tool table, the gates, the server-only arguments and the embedded part are Himalaya's.

### Requirement: An MCP server over stdio
`himalaya mcp` SHALL run an MCP server (revision 2025-06-18) over stdio for one account and one `--backend` on pimalaya-cli's shared MCP core, answering `initialize`, `ping`, `tools/list` and `tools/call`, one JSON-RPC 2.0 message per line, with stdout carrying protocol messages only. A line that is not UTF-8 or not JSON SHALL answer -32700 and one that is not a JSON object, a batch included, -32600, both with a null `id`; a request whose `jsonrpc` is not `"2.0"` SHALL answer -32600 with its `id`; the server SHALL read on after each. `initialize` SHALL answer the client's `protocolVersion` when it is 2025-06-18, and 2025-06-18 otherwise. Its commands SHALL run over a session kept for the server's lifetime, as the REPL's do, and SHALL never read their input from stdin. A failure of the server itself, resolving its account included, SHALL print as text on stderr and exit 1, a person missing a configuration meeting the welcome instead (cairn/spec/config.md).

#### Scenario: A batch
- GIVEN a server
- WHEN the line `[{"jsonrpc":"2.0","id":1,"method":"ping"}]` is read, then a `ping` with id 4
- THEN the first answers -32600 with `id` `null` and the second `{}`

### Requirement: The server tells the agent how to use it
`initialize` SHALL carry `instructions` saying how to find a mailbox (`mailbox_list`, the `mailbox` argument, the inbox when left out) and an envelope id (`envelope_list`, `envelope_search` and its query language), that `attachment_download` returns the bytes as an embedded resource and the composers take files inline as `attach-data`, that `--allow-write` and `--allow-send` gate the tools and arguments that change the store and send mail, and that a failed call answers `isError` with the `himalaya-error` object. Every tool name it holds SHALL be one of the tool table.

### Requirement: Tools are the classified unified API
The server SHALL expose one tool per leaf command of the unified API that is classified as reading, writing or sending and that the account's transports serve at least one capability of, as the static tier knows without connecting, named `<group>_<leaf>`, and no protocol-specific or raw passthrough command. A tool whose capability only the server decides SHALL stay listed and answer `capability-missing` as a tool error when the server lacks it. An argument reaching a capability of its own (the composers' `send` and `no-save` reach `submit`, their `save` reaches `message.add`) SHALL be left out of the tool where the static tier says the account does not serve it, the tool staying listed.

#### Scenario: A send-only account
- WHEN `himalaya mcp --allow-write --allow-send` serves an account configuring SMTP alone
- THEN `tools/list` holds `message_send` and `message_compose`, and no `envelope_list` or `flag_add`

### Requirement: The search tool says what the backend answers
The `envelope_search` description SHALL end with the clauses and sort keys of the query language the account's storage backend answers fully, in part with each detail, and not at all, from the capability tables.

### Requirement: A tool's input is the command's grammar
A tool's `inputSchema` SHALL be generated from its clap command by pimalaya-cli's `mcp::grammar::Grammar`, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command from the argv the grammar writes. The one exception SHALL be the server-only arguments its tool declares, which clap never sees. An argument the `inputSchema` does not declare SHALL be refused before the command runs as a tool error ``Unknown argument `<key>`, <tool> takes <properties>``, the server-only ones named among them. A value of the wrong type, and a count other than one for an argument the tool takes one value of, SHALL be refused the same way. Each of these refusals, and a value the command's grammar refuses once the argv is parsed (a negative `page-size`), SHALL carry the code `usage-invalid`, the last with clap's message without its `error: ` prefix. A tool's property takes a lone value where the command repeats one, so `query` is a string or an array of words; `page` and `page-size` are integers; `attachment_download` takes `attachment-id` as one string.

### Requirement: A server-only argument is declared on its tool
An input the server takes and the command's grammar does not SHALL be declared on its tool in the tool table, with its JSON Schema, and nowhere in the clap grammar. It SHALL exist only where the CLI already says the same thing another way, through a path or stdin that the server keeps closed. It SHALL be a property of that tool's `inputSchema`, listed whenever the tool is. The server SHALL take it off the call before the argv is rebuilt and validate it, refusing a bad value as a tool error with a stable code before the command runs. The validated value SHALL reach the parsed command through a field clap skips (`#[arg(skip)]`), which no command line can set.

### Requirement: A tool's output is the command's payload
A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is typed `object` and the tool has one output shape, save `attachment_download`, whose bytes travel as an embedded resource instead. The events a command prints SHALL come first in `content`, one text item each, the result last. A command ending with a partial outcome (a refused IMAP, SMTP or ManageSieve completion, a failed `account check`) SHALL keep its result as `structuredContent` and set `isError: true`. A refused or failed call, a command printing no value or several (`The command printed <n> values, one expected`) and a part tool whose command printed no part (`The command printed no part to embed`) included, SHALL return `isError: true` with the `himalaya-error` object and no `structuredContent`; an unknown tool SHALL be a JSON-RPC error.

### Requirement: Writing and sending are opt-in
Without flags the server SHALL list reading tools only, without the arguments that write. `--allow-write` SHALL add the tools and arguments that change the store, `mailbox_create`, `mailbox_rename` and `mailbox_delete` among them. `--allow-send` SHALL add those that send mail, `vacation_set` among them, an auto-reply once on answering mail. Under `--allow-send` a sent message MAY carry `attach-data`, so the flag lets an agent send arbitrary binary data and not only text.

### Requirement: A tool reads and writes no local file
No tool SHALL take a local path: `attachment download` SHALL be exposed only with its fixed `--stdout`, which writes no file, `message parse` SHALL NOT be exposed, path arguments SHALL NOT be properties, and a message argument SHALL be taken as the message itself even when it spells an existing path, and refused when it opens on no header field.

### Requirement: The composers attach data given inline
`message_compose`, `message_reply` and `message_forward` SHALL take the server-only argument `attach-data`, an array of objects `{filename, mimeType, data, text}`, with `filename` required and exactly one of `data`, in base64 (RFC 4648 §4, padding optional, no whitespace), or `text`, in UTF-8 and attached as given. Each item SHALL become an attachment part after those of `--attach`, built as theirs are: its decoded bytes, its filename (RFC 2231 encoded when not ASCII) and `mimeType`. When the item has no `mimeType`, it SHALL be guessed from the filename as `--attach` guesses it from a path, `application/octet-stream` when unknown. Nothing SHALL be read from disk or written to it. A filename that is missing, empty or holds a control character, an item with both or neither of `data` and `text`, bad base64, a `mimeType` that is no bare `type/subtype` of RFC 2045 tokens, and an unknown key SHALL be refused with the code `attachment-invalid`, naming the item as `attach-data[<index>]` and its filename, nothing being composed, saved or sent.

#### Scenario: A report saved with its attachment
- WHEN `message_compose` is called under `--allow-write` with `save: "inbox"` and `attach-data: [{"filename": "report: q3.txt", "text": "ok"}]`
- THEN the stored message holds a `text/plain` attachment named `report: q3.txt` whose bytes are `ok`

### Requirement: An inline attachment is capped before it is decoded
`himalaya mcp --max-attachment-size <BYTES>`, 10485760 (10 MiB) by default, SHALL bound each item of `attach-data`. An item over it SHALL be refused with the code `attachment-too-large` before anything is decoded: the size of `data` is estimated from its length (three bytes per four characters, less the padding), the size of `text` is its UTF-8 byte length. The flag SHALL NOT be a tool property.

#### Scenario: Oversized and malformed
- WHEN `attach-data` carries a `data` string longer than the cap allows that is not base64
- THEN the call answers `attachment-too-large`, not `attachment-invalid`, and nothing is stored

### Requirement: An attachment is read as an embedded resource
`attachment_download` SHALL be a reading tool always run with `--stdout`, appended to its argv as a fixed argument, `stdout`, `dir` and `inline` not being properties and `attachment-id` taking exactly one id, as a string. Its `structuredContent` SHALL be the part the command prints without `data`, the text block the same JSON, and its `outputSchema` the part's schema without `data`. The part's bytes SHALL follow in `content` once, as an embedded resource whose `blob` is their base64, `mimeType` the part's type and `uri` `himalaya://<account>/<mailbox>/<message-id>/<part-id>`, each segment percent-encoded and the mailbox as the call names it, `inbox` when omitted.

#### Scenario: A binary attachment
- WHEN `attachment_download` is called with a message id and the id of a binary part
- THEN `structuredContent` holds the part's id, type, file name and size and no `data`, and the embedded resource's `blob` decodes to the part's exact bytes

### Requirement: The server bounds the part a tool reads
The same `--max-attachment-size` SHALL bound the decoded size of the part a tool reads. A larger part SHALL be refused before it is encoded, with the code `attachment-too-large` and `isError: true`.
