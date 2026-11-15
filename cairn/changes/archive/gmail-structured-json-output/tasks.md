---
cairn: tasks
change: gmail-structured-json-output
---

# Tasks

- [x] Gmail messages/drafts/threads `get` and `history list`: hand-written output types replacing the `Message` wrapper, including `GmailMessageHeaderOutput` for the fetched headers.
- [x] Gmail settings readers: transparent newtypes over the io-gmail resource for delegates, forwarding addresses, filters and send-as; hand-written for the vacation, imap, pop, language and auto-forwarding singletons, whose io-gmail types lack `JsonSchema`.
- [x] Microsoft Graph `message get`: transparent newtype over `MsgraphMessage`, matching what `message list` already emits.
- [x] Drop the fake `HistoryOutput` schema type that documented the string wrapper.
- [x] `messages get` and `threads get`: narrow headers client-side through a shared `message_headers`, case-insensitive, order and repeats preserved.
- [x] `drafts get --format raw`: decode and write the RFC 5322 bytes, like `messages get --format raw`.
- [x] Register all fourteen new schemas in src/json_schema.rs; always emit Vec fields so the schema matches the payload.
- [x] Tests for the header filter; changelog.
- [x] Build/test/fmt/clippy.
- [x] Fold into cairn/spec/commands.md; log; land.
