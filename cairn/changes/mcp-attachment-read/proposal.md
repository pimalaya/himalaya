---
cairn: change
id: mcp-attachment-read
status: landed
created: 2026-11-06
---

# `himalaya mcp`: read an attachment's content without touching disk

## Why

An agent can list a message's attachments (`attachment_list`, `message_read`) but cannot read one (himalaya#805). `attachment download` is left out of the server because it writes files, and cairn/changes/mcp-server kept "download bounded to a configured directory" for later. A directory would still hand the agent a path it cannot open when the client runs on another machine, and it is a write to disk the agent triggered.

The CLI already has the piece that fits: `attachment download --stdout --json` prints one part as `{id, mime, filename, size, data}`, `data` in base64, and writes nothing (`PartBytes`, src/shared/attachment/download.rs).

## What

1. **Fixed arguments.** A tool in src/mcp.rs `TOOLS` can append fixed arguments to its argv, before the `--` that opens the positionals; an argument a tool fixes is not a property. `attachment_download` is a reading tool always run with `--stdout`: no MCP-only verb, no new `attachment read` leaf, the grammar stays the command's. `dir` stays hidden as a path, and `attachment-id` takes exactly one id (`minItems` and `maxItems` 1, required), the command itself refusing any other count.
2. **The bytes travel once.** `structuredContent` is the `PartBytes` object without `data`, and the text block is that same JSON. The bytes follow in `content` as an embedded resource, `{type: "resource", resource: {uri, mimeType, blob}}`, `blob` being the `data` the command printed, already base64, and the URI `himalaya://<account>/<mailbox>/<message-id>/<part-id>`, each segment percent-encoded, the mailbox as the call names it (`inbox` when omitted).
3. **Output schema.** The registered `himalaya-attachment-download` schema has two shapes (the written table or the part), so the generic rule declares none. Under the fixed `--stdout` the tool has one shape, so its `outputSchema` is the `PartBytes` schema without `data`, which `structuredContent` conforms to.
4. **A server-side cap.** `himalaya mcp --max-attachment-size <BYTES>` (10 MiB by default) bounds the part a tool reads. It is a server flag, never a tool property, so the model cannot raise its own guard. A larger part is refused after decoding and before encoding, with the new error code `attachment-too-large` (`isError: true`), so a large video never lands in the model's context nor blocks the pipe. Outside the server nothing is bounded.
5. **No resources capability.** MCP 2025-06-18 says a server that embeds resources SHOULD declare `resources`, not MUST. Declaring it would promise `resources/list` and `resources/read`, and an empty list with a refused read is a promise unkept, so the server keeps declaring tools alone. The embedded resource carries its bytes and needs no read. `resources/read` answering a part URI stays under Later; the URI is shaped so it maps back onto the same call.
6. **Still no local file.** `--stdout` writes nothing, so the "no local file" requirement names the fixed `--stdout` download instead of excluding `attachment download`. `message parse` stays excluded.

## Later

- The `resources` capability: an empty `resources/list` and `resources/read` for a part URI.
