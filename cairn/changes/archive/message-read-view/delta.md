---
cairn: change
change: message-read-view
---

# Delta

## ADDED Requirements

### Requirement: A read message is a designed view
`message read --json` SHALL print `{headers, text, html, parts}`, the same on every backend, rather than the parser's own model. `headers` SHALL carry `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}`, `subject` decoded, `date` as RFC 3339 with its offset or `null`, `messageId`, `inReplyTo` and `references` without angle brackets, and `listId`, `listUnsubscribe`, `precedence` and `autoSubmitted` as found. `text` SHALL be every `text/plain` body part decoded to UTF-8, joined by a blank line, and `html` the first `text/html` body part, each `null` when absent.

### Requirement: Every part has an id and a role
`parts` SHALL list every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`, the `id` being the one `attachment list` and `attachment download` take. A `multipart/*` SHALL be no part; `message/rfc822` or `Content-Disposition: attachment` SHALL be an `attachment`; a `text/plain` or `text/html` without a filename SHALL be a `body`; an `image/*` marked inline or carrying a `Content-ID` SHALL be `inline`; anything else SHALL be an `attachment`.

### Requirement: A message too complex is refused whole
A message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines, SHALL be refused with the JSON error code `message-too-complex` rather than read in part.

### Requirement: A part can be read without touching the disk
`attachment download <MESSAGE-ID> <PART-ID> --stdout` SHALL write that part's decoded bytes to stdout and touch no file; under `--json` it SHALL print `{id, mime, filename, size, data}`, `data` in base64.

## MODIFIED Requirements

(`message read --json` no longer prints the parser's model; `attachment list --json` gains `contentId`.)
