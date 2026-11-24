---
cairn: log
change: the-live-night-findings
landed: 2026-11-24
---

# IMAP Message-ID search sends the bracketed id

Found on 2026-11-23 by the live night (.lanes/live/report.md): a `mid:` search over IMAP found nothing on Stalwart and iCloud.

## Evidence (2026-11-24, journaled in .lanes/live/journal.md)

- iCloud matches `HEADER Message-ID` on the whole bracketed value alone: `<id>` finds the message, the bare id and any fragment find nothing.
- Fastmail (Cyrus) matches both forms, as RFC 3501's substring match does.
- Stalwart 0.16 answers `HEADER` on `From`, `To` and `Subject` but on no other field tried, Message-ID in any form included, and its `TEXT` does not find a whole Message-ID: no IMAP form reaches it there.

## What landed

- Every IMAP `HEADER Message-ID` key sends the id in angle brackets: the `mid:` clause and the UID lookup of an `APPEND` answered without `APPENDUID` (src/imap/backend.rs, `message_id_key`).
- tests/imap_message_id.rs: a scripted server matching the bracketed form alone.

## Capabilities moved

- search: the IMAP `message-id` clause sends `<id>`.
- provider-quirks: Message-ID header search takes the bracketed id; Stalwart's gap recorded.
