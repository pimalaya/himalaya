---
cairn: change
id: mcp-inline-attachments
status: landed
created: 2026-11-06
---

# MCP composers attach files given inline

## Why

An agent driving `himalaya mcp` cannot attach a file (himalaya#806). The composers' `--attach` takes a path, and paths are closed to tools (cairn/spec/mcp.md, "A tool reads and writes no local file"), so the report an agent just wrote cannot go out with the mail it composes. A CLI `--attach-data` flag was rejected: nobody types base64 into a shell, and a script that has the bytes writes them to a file and uses `--attach`. So the input is the server's alone, and the server now needs an input the CLI does not have. That is settled once, as a rule for every such input, rather than as an exception for this one.

## What

1. **Server-only arguments, one rule.** An argument the server takes but the command's grammar does not is declared on its tool in the tool table (src/mcp.rs `TOOLS`), with its JSON Schema, and nowhere in clap. It is added to that tool's `inputSchema` and gated with the tool. The server takes it off the call before the argv is rebuilt, so clap never sees it. The server validates it, and a bad value is a tool error with a stable code, before the command runs. The validated value is handed to the parsed command through a field clap skips (`#[arg(skip)]`), so no command line reaches it. Such an argument is allowed only where the CLI already says the same thing another way, through a path or stdin that the server keeps closed. An argument that adds a behaviour the CLI lacks belongs in the grammar. "A tool's input is the command's grammar" is modified to name this class.
2. **`attach-data`.** It is on `message_compose`, `message_reply` and `message_forward`, and listed whenever its composer is. It is an array of `{filename, mimeType?, data?, text?}`, with exactly one of `data` (base64, the RFC 4648 standard alphabet, padding optional, no whitespace) or `text` (UTF-8, attached as given, so the model need not base64 the text reports it usually attaches). The value is a typed object rather than `NAME:MIME:BASE64`, because a filename can hold a colon. `mimeType` is optional, guessed from the filename as `--attach` guesses it from the path (`mime_guess`, `application/octet-stream` when unknown), and must be a bare `type/subtype`. Each item becomes a part through the same builder `attachment` function the paths go through, after them. No disk is read or written, and signing, encryption and templates stay with mml.
3. **Refusals.** A filename that is missing, empty or holds a control character, both or neither of `data` and `text`, bad base64, a malformed `mimeType` and an unknown key are all refused as `attachment-invalid`. Each refusal names the item as `attach-data[<index>]` with its filename. Nothing is composed, saved or sent.
4. **A cap refused before decoding.** `himalaya mcp --max-attachment-size <BYTES>` (10 MiB by default, 10485760) bounds each item, refused as `attachment-too-large`. The size of `data` is estimated from the base64 length (three bytes per four characters, less the padding), the size of `text` is its UTF-8 byte length, and both checks run before anything is decoded. The flag and the code are the ones himalaya#805 introduces for `attachment_download`, defined the same way, so the second of the two changes to land only dedupes them. The cap is **per attachment**, the same meaning the flag has in #805, where it bounds the one part a download returns. A total would be a second quantity under the same name. The JSON-RPC line holding several attachments is bounded by nothing more than today, and a `--max-message-size` is left for later.
5. **`--allow-send` sends arbitrary data.** Under `--allow-send`, an agent could already put text into a body. With `attach-data` it can send arbitrary binary data, and much more of it per message, which is a wider exfiltration channel than bodies. This is accepted under `--allow-send`: the flag already means the agent may send anything it can read, and the per-attachment cap limits how much one item carries. A server that must not send stays without the flag. `attach-data` is not gated by `--allow-send` itself: on a composer listed without `send` or `save`, it only shapes the message the tool prints.

## Later

- A `--max-message-size` bounding the whole composed message, if the per-attachment cap proves too loose.
- `charset=utf-8` on a `text` attachment of a `text/*` type, which `--attach` leaves out for files too.

## Not in scope

A CLI flag, which `--attach` already covers. Reading attachments out (`attachment_download`) is himalaya#805.
