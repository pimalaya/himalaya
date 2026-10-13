---
cairn: log
change: imap-copy-counts
landed: 2026-10-13
---

# IMAP copy and move counts without a readable COPYUID

The fix for [#781] (Mailo answering `UID COPY` with `COPYUID 0 …`, a UIDVALIDITY RFC 4315 forbids) made a tagged `OK` without a readable `COPYUID` count as every requested message copied. A UIDPLUS server sends no `COPYUID` when nothing matched, and RFC 9051 §6.4.9 answers such a `UID COPY` or `UID MOVE` with a plain `OK`, so moving a message that no longer exists was reported as done instead of `No message moved: no id matched`. MOA's batch classification test on Stalwart caught it.

himalaya cannot tell a missing `COPYUID` from an unreadable one: imap-codec turns `COPYUID 0 …` into an opaque `Code::Other`, and io-imap's copy and move coroutines keep only `Code::CopyUid`, reporting both as `None`. The source is asked instead: `copy_messages` and the copy fallback of `move_messages` follow a `UID COPY` without `COPYUID` with a `UID SEARCH UID` of the requested set, which `COPY` leaves untouched, and count, flag and expunge only what it finds. `UID MOVE` empties the source, so with MOVE the search comes first and only the UIDs present are moved, none being sent when nothing is there. A valid `COPYUID` still answers alone, without the extra round trip on the copy paths.

Covered by a scripted IMAP server in `src/imap/backend.rs` (valid `COPYUID`, UIDVALIDITY 0, none from a UIDPLUS server, none without UIDPLUS, with and without MOVE) and by MOA's `servers/mail/src/write.test.ts` on Stalwart.

## Capabilities moved

- [backends](../spec/backends.md): *IMAP move without the MOVE extension* flags and expunges what was really copied; added *IMAP copy and move counts*.

[#781]: https://github.com/pimalaya/himalaya/issues/781
