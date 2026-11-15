---
cairn: log
change: next-page-prints-after-the-listing
landed: 2026-11-15
---

# Next page prints after the listing

Line X14 of the family's cli-alignment-2026-11-15 plan, ruling D27.

## What landed

- pimalaya-cli 0.6.0, io-imap 0.11.2 and io-msgraph 0.9.2; Cargo.lock moved those crates alone.
- Every listing prints `Next page:` after its items, one blank line below them and above the item errors, its header values two spaces past the longest label; the text snapshots follow.

## Capabilities moved

- [commands](../spec/commands.md): *Text output is plain text* modified.
