---
cairn: change
id: an-imap-group-is-no-address
status: landed
created: 2026-11-18
---

# An IMAP group is no address

## Why

Finding HIM-22 of the family's cli-family-analysis-2026-11-15 plan: IMAP ENVELOPE group markers printed as addresses, `To: undisclosed-recipients:;` listing as `[{"email":"undisclosed-recipients"},{"email":""}]` where Maildir and mbox print `[]`, and the conversion existed twice (`envelope list`, `imap fetch`) with two decodings.

## What

One `imap::utils::to_addresses` leaves out the markers (RFC 3501 §7.4.2, a `NIL` host), used by both.
