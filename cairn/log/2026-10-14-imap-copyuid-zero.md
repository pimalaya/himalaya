---
cairn: log
change: imap-copyuid-zero
landed: 2026-10-14
---

# Mailo's COPYUID read despite its UIDVALIDITY 0

io-imap now reads a `COPYUID` whose UIDVALIDITY is `0`, such as Mailo's `OK [COPYUID 0 1582 22704] UID COPY done` ([#781]), which imap-codec leaves as an opaque `Code::Other`; any other deviation from the RFC 4315 grammar still gives no `COPYUID`. `copy_messages` and the copy fallback of `move_messages` therefore count, flag and expunge the source UIDs that `COPYUID` lists, without the `UID SEARCH UID` that 2026-10-13's [imap-copy-counts](2026-10-13-imap-copy-counts.md) sent for it. The search stays for a missing or otherwise malformed `COPYUID`, and before a `UID MOVE`, which empties the source before any `COPYUID` can be read.

Covered by the scripted IMAP server in `src/imap/backend.rs` (UIDVALIDITY 0 now answers alone; a `COPYUID` with a UID 0 still falls back to the search) and by io-imap's copy and move unit tests.

## Capabilities moved

- [backends](../spec/backends.md): *IMAP copy and move counts* counts a `COPYUID` with UIDVALIDITY 0.

[#781]: https://github.com/pimalaya/himalaya/issues/781
