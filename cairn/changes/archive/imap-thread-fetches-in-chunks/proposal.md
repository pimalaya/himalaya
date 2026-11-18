---
cairn: change
id: imap-thread-fetches-in-chunks
status: landed
created: 2026-11-18
---

# imap thread fetches in chunks

## Why

Finding HIM-23 of the family's cli-family-analysis-2026-11-15 plan, as amended on 2026-11-16: `imap thread` fetched every threaded message in one `FETCH`; consecutive ids fold into ranges, but a scattered set made an 84 KB command line for 15,000 messages, past Dovecot's documented 64 KB.

## What

255 ids a `FETCH`, as io-imap's sort fallback chunks its own.
