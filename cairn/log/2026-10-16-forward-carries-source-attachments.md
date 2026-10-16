---
cairn: log
change: forward-carries-source-attachments
landed: 2026-10-16
---

# A forward carries the attachments of its source

Requested in [#780], contributed in [#788]. `message forward` quoted the text body and dropped everything else, so the recipient got a quote of a message whose files they could not read. It now appends the source's attachment parts after the quoted body and the `--attach` files, read through the shared `message::part` walker, so the quote and the posting style compose exactly as before. Multipart containers are skipped, since a `name` on one would otherwise pass as an attachment with no bytes.

Nameless parts are named rather than dropped: an attached email (`message/rfc822`, usually nameless) from its subject plus `.eml`, anything else `attachment` plus the extension of its MIME type. `--no-attachments` carries none and combines with `--attach`. A reply carries nothing.

Covered by `tests/message_forward.rs` on Maildir. Not exercised against a remote backend.

Capabilities moved: **commands** (A forward carries the attachments of its source).

[#780]: https://github.com/pimalaya/himalaya/issues/780
[#788]: https://github.com/pimalaya/himalaya/pull/788
