---
cairn: change
id: the-raw-reply-keeps-its-literals
status: landed
created: 2026-11-18
---

# The raw reply keeps its literals

## Why

Finding HIM-18 (local part) of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. io-imap returns the `imap raw` reply through `from_utf8_lossy`, where an octet that is no UTF-8 grows into a three-byte U+FFFD, and src/imap/raw.rs then skipped a literal by its announced octet count over that string. A Latin-1 or KOI8-R body landed the cursor inside a replacement character and panicked ("not a char boundary", exit 101), or resumed early and cut the literal's tail into responses, so a body line `A1 NO forged` came out as a tagged completion: the sender of a mail chose what the structured result said. The schema half (io-imap returning bytes, a non-UTF-8 literal printed as base64) waits on io-imap and is not part of this change.

## What

- `split_responses` returns the responses it framed and, when a literal holds a U+FFFD or its announced end falls inside a character, the rest of the reply from the response announcing it, unsplit: a literal holding one has no knowable octet count.
- `split_reply` prints that rest as one `unparsed` event (one block in text); `imap raw` warns and exits 3, the completions after it unread.
