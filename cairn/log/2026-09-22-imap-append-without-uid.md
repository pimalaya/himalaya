---
cairn: log
date: 2026-09-22
change: imap-append-without-uid
---

# Accept a successful IMAP append without a recoverable UID

`message send --save` saves before it sends. An IMAP server that acknowledged the append but did not return `APPENDUID` therefore made Himalaya recover the new UID by searching for the submitted `Message-ID`. QQ Exmail rewrites that header, so the search returned no match, Himalaya reported an error, and SMTP never ran even though the saved copy existed.

## What landed

The shared add result now carries an optional backend id. IMAP returns no id when an acknowledged append has neither `APPENDUID` nor a fallback search match; the other storage backends keep returning their authoritative ids. Text output reports that the id is unavailable, JSON emits `null`, and save-then-send continues. A rejected `APPEND` still fails before any send.

The fallback keeps choosing the highest UID when it does find matches, preserving the previous duplicate-`Message-ID` behaviour.

## Capabilities moved

- backends: added the successful-append-without-a-recoverable-id rule to *Append and search gaps*

## Verification

The reduced `imap,smtp,rustls-ring` suite passes all 96 tests, and the all-feature suite passes all 134 tests. Regression tests cover an empty fallback result, selection of the highest recovered UID, the success text for an unavailable id, and JSON `null` serialization.

Clippy with warnings denied reaches only three pre-existing lints in untouched Gmail and wizard files under Rust 1.98; it reports no changed-file diagnostic. The repository-wide formatter likewise finds a pre-existing difference in `src/gmail/search.rs`; every touched Rust file passes `rustfmt --check` on its own.
