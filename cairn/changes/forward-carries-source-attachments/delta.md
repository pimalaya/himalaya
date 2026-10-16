---
cairn: delta
change: forward-carries-source-attachments
---

# Delta

## ADDED Requirements

### Requirement: Forward carries the attachments of the source
`message forward` SHALL carry every part of the source disposed as an
attachment (`Content-Disposition: attachment`) into the composed
forward as an attachment part of its own, under the file name the
source gave it, with its content type and decoded bytes. The quoted
body SHALL be composed exactly as it is without attachments: the
carrying appends parts and SHALL NOT restructure the body, drop the
quote or change the posting style. A nameless `message/rfc822` part SHALL
use its decoded subject, sanitized for filesystem use, with an `.eml`
extension, falling back to `attachment.eml` without a usable subject.
Other nameless parts SHALL use `attachment` plus an extension known to
`mime_guess` (`.bin` for `application/octet-stream`), or just `attachment`
for an unknown MIME type. A source with no attachment part SHALL compose
as it did before.

### Requirement: Forward can leave the source attachments out
`message forward --no-attachments` SHALL compose the forward without
carrying any part of the source, leaving the quoted body alone, the
flag combining with `--attach` for a writer replacing the source's
attachments with files of their own. A reply SHALL never carry the
attachments of its source.
