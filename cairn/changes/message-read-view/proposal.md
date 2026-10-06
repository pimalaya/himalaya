---
cairn: change
id: message-read-view
status: active
created: 2026-10-06
---

# A designed JSON view for `message read`, and a part's bytes on stdout

## Why

`message read --json` prints mail-parser's own model as it stands (`MessageView`, `#[serde(transparent)]`, schema `serde_json::Value`): a shape nobody designed, that follows mail-parser's releases and that the published schema says nothing about. A client wanting a message's headers, bodies and parts therefore reads `--raw` and parses the MIME again on its own: MOA carries 900 lines of MIME reading for that (headers, RFC 2047 and 2231, nested parts, transfer encodings, charsets). calendula's event projection shows the other way: a view designed once, the same on every backend, that clients read as is.

The bytes of one part have no way out either: `attachment download` writes files into a directory. A client that must not touch the disk (MOA's mail server never does) cannot use it, and `--raw --json` turns a raw message into lossy UTF-8 text.

## What

- `message read --json` prints a designed view, the same on every backend:
  - `headers`: `from`, `to`, `cc`, `bcc`, `replyTo` and `sender` as lists of `{name, email}` (`name` decoded, `null` when absent); `subject` decoded; `date` as RFC 3339 with its offset (`null` when unreadable); `messageId`, `inReplyTo`, `references` (ids without angle brackets); `listId`, `listUnsubscribe`, `precedence`, `autoSubmitted` as found (decoded text or `null`).
  - `text`: every `text/plain` body part decoded to UTF-8, in order, joined by a blank line; `html`: the first `text/html` body part decoded; each `null` when absent.
  - `parts`: every leaf part, depth first, as `{id, role, mime, filename, size, contentId, charset, method}`: `id` is the one `attachment list` and `attachment download` use; `role` is `body`, `inline` or `attachment`; `filename` decoded (RFC 2231, RFC 2047), `null` when absent; `size` the decoded byte count; `contentId` without angle brackets; `method` the `text/calendar` part's `method` parameter, uppercased.
  - The role rule: a `multipart/*` is no part; `message/rfc822` or `Content-Disposition: attachment` is an attachment; a `text/plain` or `text/html` without a filename is a body; an `image/*` marked inline or carrying a `Content-ID` is inline; anything else is an attachment.
- Bounds: a message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines, is refused with the JSON error code `message-too-complex` rather than read in part.
- `attachment download <MESSAGE-ID> <PART-ID> --stdout` writes that one part's decoded bytes to stdout, raw, and nothing else; it touches no file. Under `--json` it prints `{id, mime, filename, size, data}`, `data` in base64.
- `attachment list --json` gains `contentId`, and its ids stay those of `parts`.
- Text output of `message read` is unchanged. The old transparent JSON goes: noted as a breaking change in the changelog.

## Out

Sanitizing what a message says (hidden HTML, invisible characters, a length bound) stays with the client: it is policy, not reading. Composing and sending are untouched.
