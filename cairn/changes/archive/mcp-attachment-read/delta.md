---
cairn: delta
change: mcp-attachment-read
---

## ADDED Requirements

### Requirement: An attachment is read as an embedded resource
`attachment_download` SHALL be a reading tool always run with `--stdout`, appended to its argv as a fixed argument, `stdout` and `dir` not being properties and `attachment-id` taking exactly one id. Its `structuredContent` SHALL be the part the command prints without `data`, the text block the same JSON, and its `outputSchema` the part's schema without `data`. The part's bytes SHALL follow in `content` once, as an embedded resource whose `blob` is their base64, `mimeType` the part's type and `uri` `himalaya://<account>/<mailbox>/<message-id>/<part-id>`, each segment percent-encoded and the mailbox as the call names it, `inbox` when omitted.

#### Scenario: A binary attachment
- WHEN `attachment_download` is called with a message id and the id of a binary part
- THEN `structuredContent` holds the part's id, type, file name and size and no `data`, and the embedded resource's `blob` decodes to the part's exact bytes

### Requirement: The server bounds the part a tool reads
`himalaya mcp --max-attachment-size <BYTES>` SHALL bound the decoded size of the part a tool reads, 10 MiB by default, and SHALL NOT be a tool property. A larger part SHALL be refused before it is encoded, with the code `attachment-too-large` and `isError: true`.

## MODIFIED Requirements

### Requirement: A tool's output is the command's payload
A successful call SHALL return the command's JSON value as `structuredContent` and as text, described as `outputSchema` by its registered schema when that schema's root is an object and the tool has one output shape, save `attachment_download`, whose bytes travel as an embedded resource instead. The events a command prints SHALL come first in `content`, one text item each, the result last. A command ending with a partial outcome (a refused IMAP, SMTP or ManageSieve completion, a failed `account check`) SHALL keep its result as `structuredContent` and set `isError: true`. A refused or failed call SHALL return `isError: true` with the `himalaya-error` object; an unknown tool SHALL be a JSON-RPC error.

### Requirement: The JSON contract is described and versioned (cairn/spec/commands.md)
Unchanged but for one row of the code table, after `capability-missing`:

| Code | Raised when |
|---|---|
| `attachment-too-large` | a part an MCP tool reads is larger than the server's `--max-attachment-size`; it is refused before being encoded, nothing of it returned |

### Requirement: A tool reads and writes no local file
No tool SHALL take a local path: `attachment download` SHALL be exposed only with its fixed `--stdout`, which writes no file, `message parse` SHALL NOT be exposed, path arguments SHALL NOT be properties, and a message argument SHALL be taken as the message itself even when it spells an existing path.
