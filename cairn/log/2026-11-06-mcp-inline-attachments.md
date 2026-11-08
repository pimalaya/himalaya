---
cairn: log
change: mcp-inline-attachments
landed: 2026-11-06
---

# MCP composers attach files given inline

`message_compose`, `message_reply` and `message_forward` take `attach-data`, an array of `{filename, mimeType, data, text}`, `data` in base64 or `text` as UTF-8 (himalaya#806). It is MCP-only: the CLI keeps `--attach`, and `message compose --attach-data` is refused by clap.

It is the first server-only argument, under a rule written once in cairn/spec/mcp.md rather than as an exception. Such an argument is declared on its tool in src/mcp.rs `TOOLS` (`McpTool::server`, a `ServerArg` with its schema and a `hand` function) and added to the tool's `inputSchema`. `McpToolSpec::split` takes it off the call before `argv` is rebuilt, so clap never sees it. Its `hand` then validates it and sets it on the parsed command through an `#[arg(skip)]` field (`attach_data` on the three composers), which no command line reaches. "A tool's input is the command's grammar" now names that exception, and an unknown key's refusal lists the server-only arguments with the others.

Each item becomes an `InlineAttachment` and goes through the builder's `attachment` function after the `--attach` files. A missing `mimeType` is guessed by the same `mime_for` that `--attach` uses. Nothing touches the disk.

A malformed item is refused under the new code `attachment-invalid`, naming it as `attach-data[<index>]` with its filename. That covers a missing, empty or control-character filename, both or neither of `data` and `text`, bad base64, a `mimeType` that is not a bare `type/subtype`, and an unknown key. The control-character check matters: without it, a filename holding CRLF was accepted and stored.

`himalaya mcp --max-attachment-size` (10485760 by default) caps each item under `attachment-too-large`, before decoding. The size of `data` is estimated from its length less the padding, so a file exactly at the cap passes. The size of `text` is its UTF-8 byte length. The flag and the code are those himalaya#805 introduces for `attachment_download`, defined the same way, and the second of the two to land dedupes them. The cap is per attachment, which is the flag's meaning in #805.

Under `--allow-send`, an agent can now send arbitrary binary data, which is accepted under that flag.

Verified: 487 tests pass with all features, 474 on master before the change (7 new unit tests in src/mcp.rs, 6 new tests over stdio in tests/mcp.rs). Clippy is clean with `-D warnings` and `cargo fmt --check` passes. `cargo doc` passes with broken intra-doc links denied. The `imap,smtp`, `jmap` and bare builds compile with no new warning, and the `sieve` tests pass. Deliberate breakage made the tests fail: decoding before the size check, passing the whole call to the argv, and dropping the filename control-character check.

The manual end-to-end check piped `initialize`, `tools/list` and a `message_compose` with `save` into the built binary over a temporary Maildir. The message carried 4096 random bytes as `blob: 4k.bin` and a UTF-8 text as `report é.md`. Reading them back with `attachment list` and `attachment download --stdout --json` gave both byte for byte, typed `application/octet-stream` and the guessed `text/markdown`.

Capabilities moved: **mcp** (A tool's input is the command's grammar, Writing and sending are opt-in: modified; A server-only argument is declared on its tool, The composers attach data given inline, An inline attachment is capped before it is decoded: added); **commands** (`attachment-too-large` and `attachment-invalid` join the code table).
