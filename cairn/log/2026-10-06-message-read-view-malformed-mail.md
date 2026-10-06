---
cairn: log
change: message-read-view
date: 2026-10-06
---

# The `message read` view reads malformed mail as MOA did

Run against the 13 raw messages of MOA's parity vectors (`messageFromRaw` in MOA's `evals/vectors/mail.json`), the view of 17068b4 disagreed with MOA's own reader on four malformed but common messages: UTF-8 bytes labelled `us-ascii`, a raw Windows-1252 subject, a UTF-8 character split across two encoded words, and `Content-Type: text/` with LF-only line endings. All four now read as MOA reads them.

## What landed

- src/shared/message/header.rs (new): a header read from its raw bytes (`raw_text`, `unstructured`), raw 8-bit bytes decoded as UTF-8 when valid and Windows-1252 otherwise (`eight_bit`), an RFC 2047 decoder joining adjacent encoded words of one charset as bytes and dropping the whitespace between adjacent words (`decode_words`), and a plain `name=` / `filename=` parameter read unquoted (`plain_param`), since mail-parser decodes the encoded words of a quoted parameter one by one; an RFC 2231 form still wins and is left to mail-parser.
- src/shared/message/read.rs: subject and list headers through `header::unstructured`; address lists split from the raw header (groups flattened, comments stripped, names unquoted and decoded) instead of mail-parser's addresses.
- src/shared/message/part.rs: an invalid `Content-Type` (type or subtype missing, empty or not a token) is `text/plain` with charset `us-ascii`; a part labelled `us-ascii`/`ascii` or naming no charset whose bytes are valid UTF-8 is read as UTF-8; file names re-parsed from the raw header; `role`, `filename` and `text` take the message.
- Unit tests built from the four raw messages, plus 8-bit names and file names, split encoded words in names and file names, the invalid `Content-Type` forms and the plain parameter reader.

Capabilities moved: **commands** (malformed mail reads as its sender meant).

## Verification

`cargo test --all-features`: 219 passed. `cargo clippy --all-features --all-targets` clean. `cargo fmt --check` clean. `cargo build --no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir` builds. The 13 vectors through `message read --json` on a Maildir: sender, subject, date, HTML presence, attachments (count, type, size, file name when present), reply-to and the list headers (as MOA's `bulk`) agree on all 13; the text agrees once line endings are normalized and the ends trimmed, except where MOA derives it from the HTML (msg01x, msg04x) or strips invisible characters (msg11x), which is MOA's policy.
