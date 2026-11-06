---
cairn: tasks
change: mcp-attachment-read
---

# Tasks

himalaya#805, over cairn/changes/mcp-server.

- [x] Fixed arguments in the tool table, appended before `--`, their arguments not properties; arguments taking exactly one value
- [x] `attachment_download` as a reading tool with a fixed `--stdout`, `attachment-id` one id; out of the excluded leaves
- [x] The result: `structuredContent` and text without `data`, the bytes as an embedded resource with a `himalaya://` part URI; `outputSchema` the part schema without `data`
- [x] `--max-attachment-size` on `himalaya mcp` (10 MiB), checked before encoding, error code `attachment-too-large`
- [x] Tests over stdio with a maildir fixture: listed read-only, `stdout` and `dir` absent, the bytes decoded exactly, the URI, two ids refused, the cap, nothing written (fails with the fixed `--stdout` removed), the cap no tool property; unit tests for the argv, the schema, the URI and the cap
- [x] README and CHANGELOG
- [x] Fold into cairn/spec/mcp.md; log; land

## Later

- [ ] The `resources` capability: an empty `resources/list`, `resources/read` for a part URI
