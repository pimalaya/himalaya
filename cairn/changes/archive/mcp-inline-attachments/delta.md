---
cairn: delta
change: mcp-inline-attachments
---

# Delta

Capability mcp. The codes `attachment-invalid` and `attachment-too-large` join the code table of cairn/spec/commands.md.

## MODIFIED Requirements

### Requirement: A tool's input is the command's grammar
A tool's `inputSchema` SHALL be generated from its clap command, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command. The one exception SHALL be the server-only arguments its tool declares, which clap never sees. An unknown argument SHALL be refused naming the known ones, the server-only ones included.

### Requirement: Writing and sending are opt-in
Without flags the server SHALL list reading tools only, without the arguments that write. `--allow-write` SHALL add the tools and arguments that change the store, `mailbox_create`, `mailbox_rename` and `mailbox_delete` among them. `--allow-send` SHALL add those that send mail, `vacation_set` among them, an auto-reply once on answering mail. Under `--allow-send` a sent message MAY carry `attach-data`, so the flag lets an agent send arbitrary binary data and not only text.

## ADDED Requirements

### Requirement: A server-only argument is declared on its tool
An input the server takes and the command's grammar does not SHALL be declared on its tool in the tool table, with its JSON Schema, and nowhere in the clap grammar. It SHALL exist only where the CLI already says the same thing another way, through a path or stdin that the server keeps closed. It SHALL be a property of that tool's `inputSchema`, listed whenever the tool is. The server SHALL take it off the call before the argv is rebuilt and validate it, refusing a bad value as a tool error with a stable code before the command runs. The validated value SHALL reach the parsed command through a field clap skips (`#[arg(skip)]`), which no command line can set.

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
