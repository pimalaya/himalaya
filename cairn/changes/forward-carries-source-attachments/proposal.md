---
cairn: change
id: forward-carries-source-attachments
status: landed
created: 2026-10-16
---

# Proposal: forward carries the attachments of the source

## Why

`message forward` quotes the text body and drops the source's attachments. [#780] agreed on the fix: keep the quoted body, re-attach the source's attachments under their file names, and add `--no-attachments`.

## What changes

`message forward` appends the source's attachment parts after the quoted body, read through the shared `message::part` walker. Nameless parts are named: an attached email from its subject plus `.eml`, anything else from its MIME type. `--no-attachments` opts out. A reply carries nothing.

## AI disclosure

Implemented with Hermes, reviewed with OpenCode, review follow-up prepared with Codex.

[#780]: https://github.com/pimalaya/himalaya/issues/780
