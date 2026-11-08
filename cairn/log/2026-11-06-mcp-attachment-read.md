---
cairn: log
change: mcp-attachment-read
landed: 2026-11-06
---

# `himalaya mcp`: read an attachment's content without touching disk

`attachment_download` is a reading tool of `himalaya mcp` (himalaya#805), listed without flags. It is the `attachment download` command always run with `--stdout`: the tool table (src/mcp.rs `TOOLS`) gained fixed arguments, appended to the argv before the `--` that opens the positionals (after it a flag would be a value), an argument they set not being a property. `stdout` is therefore no property, `dir` stays hidden as a path, and `attachment-id` takes exactly one id (`minItems` and `maxItems` 1, required), the command refusing any other count itself. No MCP-only verb or `attachment read` leaf was added; the description ends saying it always runs with `--stdout`.

A call's `structuredContent` and text block are the part without `data` (`{id, mime, filename, size}`), and the bytes follow once in `content` as an embedded resource, `blob` being the `data` the command printed, already base64, `mimeType` the part's type and the URI `himalaya://<account>/<mailbox>/<message-id>/<part-id>`, each segment percent-encoded (RFC 3986 unreserved kept), the mailbox as the call names it, `inbox` when omitted. The registered `himalaya-attachment-download` schema has two shapes, so its `outputSchema` is the `PartBytes` definition of that schema with `data` dropped from `properties` and `required`.

`himalaya mcp --max-attachment-size <BYTES>`, 10 MiB by default, is stored at server start as `PATHS_CLOSED` is (`mcp::max_attachment_size`, `None` outside the server), and `PartBytes::bounded` refuses a larger part after decoding and before encoding with the new error code `attachment-too-large`. It is no tool property.

The server does not declare the `resources` capability: MCP 2025-06-18 asks it of a server embedding resources as a SHOULD, and declaring it would promise `resources/list` and `resources/read`; `resources/read` for a part URI stays a later item.

`an_attachment_is_read_as_an_embedded_resource_and_nothing_is_written` snapshots every file of the test's temporary directory, `downloads-dir` included, before and after the call; with the fixed `--stdout` removed it fails on that snapshot, the command writing the part to `downloads-dir`. `a_part_over_the_server_cap_is_refused` fails with the cap bypassed.

Verified: 485 tests pass with all features (474 before; 12 in tests/mcp.rs, 5 of them new; 6 new unit tests in src/mcp.rs and src/shared/attachment/download.rs); clippy clean on all features and all targets; `cargo fmt --check`; the `imap,smtp,rustls-ring` and `jmap,rustls-ring` builds without warnings, and `cargo test --no-default-features --features sieve,rustls-ring` passing with the same warnings as master. By hand, JSON-RPC `initialize`, `tools/list` and `tools/call` piped into the built binary over a temporary Maildir account with a 10 KiB binary PDF part validate against the official 2025-06-18 schema (`InitializeResult`, `ListToolsResult`, `CallToolResult`, `EmbeddedResource`, `BlobResourceContents`), `structuredContent` against the tool's `outputSchema`, the blob decoding to the exact bytes, and no file written.

Rebased onto himalaya#807, which landed first with the same flag and code: `--max-attachment-size` and `attachment-too-large` are defined once, the flag feeding both the inline cap of `attach-data` (`McpLimits`) and the read cap (`ATTACHMENT_CAP`).

Spec: mcp (added: An attachment is read as an embedded resource, The server bounds the part a tool reads; modified: A tool's output is the command's payload, A tool reads and writes no local file); commands (the `attachment-too-large` code).
