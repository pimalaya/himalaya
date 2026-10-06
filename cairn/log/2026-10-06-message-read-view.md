---
cairn: log
change: message-read-view
date: 2026-10-06
---

# A designed JSON view for `message read`, and a part's bytes on stdout

`message read --json` printed mail-parser's own model through a transparent newtype whose schema was `serde_json::Value`, so a client wanting a message's headers, bodies and parts read `--raw` and parsed the MIME again. It now prints a view designed once, built from the raw message so every backend (IMAP, JMAP, Gmail, Microsoft Graph, Maildir, m2dir, mbox, pimdir) gives the same, and `attachment download --stdout` hands out one part's bytes without touching the disk.

## What landed

- src/shared/message/part.rs (new): the one walk over the leaf parts that `message read`, `attachment list` and `attachment download` share (`leaves`, `find`; a part id is its 1-based position in the whole part list), the role rule (`role`, `PartRole`), the part's type, decoded file name, `Content-ID`, charset and calendar method, its bytes once the transfer encoding is undone (`bytes`: a text part read back from its own byte range, since mail-parser converts it to UTF-8, and an attached message from its own range, since mail-parser keeps the whole holding message as its raw bytes), and `parse`, which refuses a message nested deeper than 8 levels, or holding more than 200 parts or 500 header lines, with the new `message-too-complex` code (src/error.rs). Depth is measured from the byte ranges of the containers still open around a part, so a truncated message cannot hide its nesting.
- src/shared/message/read.rs: `MessageReadOutput` (`headers`, `text`, `html`, `parts`) with `MessageHeaders`, `MailAddress` and `MessagePartView`, all with a real JSON Schema; the text output is unchanged and walks the shared leaves.
- src/shared/attachment/list.rs: rows built from the shared walk, with `contentId`; sizes are the bytes of `part::bytes`.
- src/shared/attachment/download.rs: `--stdout` (exactly one id, any leaf part, conflicting with `--dir`; raw bytes, or `PartBytes` under `--json`); files written with the same bytes; `AttachmentDownloadOutput` for the schema.
- src/json_schema.rs: `himalaya-message-read` and `himalaya-attachment-download` registered with the new types.
- Cargo.toml: `base64` no longer optional, the `--stdout --json` output using it on every backend.
- Unit tests: encoded words (B and Q, adjacent), RFC 2231 file names (continuation, charset) and an RFC 2047 one, ISO-8859-1 and Windows-1252 bodies, quoted-printable and base64, nested `multipart/alternative` in `multipart/mixed`, an attached `message/rfc822`, an inline image by `Content-ID`, a `text/calendar` method, address groups and invalid addresses, dates and their offsets, ids, list headers, the three bounds and their code, ids shared between `parts` and `attachment list`, `--stdout` bytes identical to the decoded part.

Capabilities moved: **commands** (a read message is a designed view; every part has an id and a role; a message too complex is refused whole, with the `message-too-complex` code; a part can be read without touching the disk).

## Verification

`cargo test --all-features`: 205 passed. `cargo clippy --all-features --all-targets` clean. `cargo build --no-default-features --features rustls-ring,imap,smtp,msgraph,pimdir` builds. Checked by hand on a Maildir fixture: `message read --json`, `attachment list -i --json`, `attachment download --stdout` (bytes compared with `cmp`) and a message of 600 header lines refused with `message-too-complex`.
