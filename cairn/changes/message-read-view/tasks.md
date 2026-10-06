---
cairn: tasks
change: message-read-view
---

# Tasks

- [x] `MessageReadOutput` (headers, text, html, parts) built from the parsed message, `JsonSchema` derived (no `serde_json::Value` schema), registered in `json-schema`.
- [x] Address lists, decoded names, RFC 3339 date, ids without angle brackets, list headers.
- [x] Bodies and parts by the role rule; part ids shared with `attachment list` and `attachment download`.
- [x] Bounds and the `message-too-complex` code.
- [x] `attachment download --stdout` (raw bytes; JSON with base64 `data`), one part only.
- [x] `contentId` in `attachment list --json`.
- [x] Same view on every backend (it is built from the raw message, so check that every backend's `message read` goes through it).
- [x] Tests: encoded words and RFC 2231 names, charsets (ISO-8859-1, Windows-1252), quoted-printable and base64, nested multiparts and `message/rfc822`, a calendar part's method, inline image by `Content-ID`, the bounds, `--stdout` bytes identical to the part.
- [x] Fold into the spec, log, changelog (breaking: the JSON of `message read`).
