---
cairn: tasks
change: mcp-inline-attachments
---

# Tasks

- [x] Rework the Cairn files to the accepted scope (MCP-only, a rule for server-only arguments), written and checked before any code
- [x] Server-only arguments in src/mcp.rs: declared on the tool in `TOOLS`, added to `inputSchema`, taken off before the argv is rebuilt, validated, handed through an `#[arg(skip)]` field
- [x] `--max-attachment-size` on `himalaya mcp`, the error codes `attachment-too-large` and `attachment-invalid`
- [x] `attach-data` on `message_compose`, `message_reply` and `message_forward`, through the builder's `attachment` function
- [x] Unit tests in src/mcp.rs: schema, stripping, validation, the cap before decoding
- [x] Tests over stdio with the maildir fixture: listed on the three composers only, exact bytes, filenames and types stored for `data` and `text`, a guessed type, each refusal with its code and nothing stored, `--max-attachment-size` not a property; no CLI flag
- [x] Manual end-to-end run: compose and save over JSON-RPC, read back with `attachment list` and `attachment download --stdout --json`
- [x] Fold the delta into cairn/spec/mcp.md and cairn/spec/commands.md, log, CHANGELOG, README; land

## Later

- [x] ~~Dedupe `--max-attachment-size` and `attachment-too-large` with himalaya#805, whichever lands second~~ Done when #805 landed second (6284d2f, 2026-11-08): one flag and one code serve both.
- [ ] `--max-message-size`, if the per-attachment cap proves too loose
