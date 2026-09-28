---
cairn: log
date: 2026-09-24
change: imap-move-without-move
---

# Move and delete on IMAP servers without MOVE

`move_messages` sent `UID MOVE` unconditionally, so on a server that does not advertise RFC 6851 MOVE both `message move` and `message delete` (which moves to the trash) failed. OVH's hosted Dovecot is one such server: it advertises `UIDPLUS` but not `MOVE`, and answers `UID MOVE` with `BAD ... command not permitted with UID`.

## What landed

**The path follows the two capabilities the session already caches.** `ImapClient::supports_move()` joins `supports_uidplus()`, and `move_messages` matches on the pair: `UID MOVE` with MOVE; otherwise `UID COPY`, `\Deleted` and, when UIDPLUS is advertised, `UID EXPUNGE` of the source UIDs `COPYUID` reported. Without UIDPLUS the expunge is skipped and logged at debug: the messages stay flagged in the source, which is recoverable and never touches unrelated `\Deleted` messages.

**The fallback reuses what the adapter already had.** `copy`, `store` and `uid_expunge` are the calls `copy_messages` and `delete_messages` make; the count comes from `COPYUID` as for a copy, and nothing is flagged or expunged when the copy affected nothing.

**Checked against OVH:** 49 messages moved across three mailboxes with the target count matching, and `message delete` taking a message to the trash and out of it with the trash count back where it started.

## Capabilities moved

- [backends](../spec/backends.md): added *IMAP move without the MOVE extension*.
