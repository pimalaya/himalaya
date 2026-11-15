---
cairn: change
id: next-page-prints-after-the-listing
status: landed
created: 2026-11-15
---

# Next page prints after the listing

## Why

Line X14 of the family's cli-alignment-2026-11-15 plan, ruling D27: `Next page:` prints after the items in every listing, and pimalaya-cli 0.6.0's `listing::Listing` does it, header values two spaces past the longest label. io-imap 0.11.2 (QRESYNC only when ENABLED lists it, a date sort ignoring a wrong weekday) and io-msgraph 0.9.2 are released beside it.

## What

- pimalaya-cli 0.6.0, io-imap 0.11.2, io-msgraph 0.9.2 in Cargo.toml and Cargo.lock (io-gpeople and io-gcal are no dependency of Himalaya).
- The text snapshots follow: `Next page:` after the items, the header block two spaces past its longest label.
