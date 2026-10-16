---
cairn: delta
change: forward-carries-source-attachments
---

# Delta

## ADDED Requirements

### Requirement: A forward carries the attachments of its source
`message forward` SHALL append every attachment part of the source (multipart containers excluded) after the quoted body, under its file name, with its MIME type and decoded bytes, the quoted body composing as it does without them. A nameless `message/rfc822` part SHALL be named from its subject, sanitized for the filesystem, plus `.eml`, or `attachment.eml`; any other nameless part SHALL be named `attachment` plus the extension of its MIME type (`.bin` for `application/octet-stream`), or bare `attachment`. `--no-attachments` SHALL carry none and combines with `--attach`. A reply SHALL never carry the attachments of its source.
