---
cairn: change
id: imap-move-without-move
status: landed
created: 2026-09-24
---

# Move and delete on IMAP servers without MOVE

## Why

The IMAP adapter's `move_messages` always sends `UID MOVE`, which is RFC 6851 and only exists on servers that advertise `MOVE`. On a server that does not, the command is rejected (Dovecot answers `BAD ... command not permitted with UID`), so `message move` fails. `message delete` fails with it: outside the trash it moves to the trash.

Such servers are still common in hosted mail. OVH's shared hosting (Dovecot, `ssl0.ovh.net`) is one: after authentication it advertises `UIDPLUS` but not `MOVE`. Moving and deleting mail are not optional there, and the only way to reach them is currently to leave Himalaya.

## What

`move_messages` picks its strategy from the capabilities the session already caches:

- **`MOVE`**: `UID MOVE`, unchanged.
- **no `MOVE`, `UIDPLUS`**: the sequence RFC 6851 §1 describes as what clients do without the extension. It runs `UID COPY` to the target, then flags the source UIDs `\Deleted` and runs `UID EXPUNGE` on exactly those UIDs. `delete_messages` already uses the same `\Deleted` plus `UID EXPUNGE` pair on the trash. The returned count comes from `COPYUID`, as it does for a copy. Nothing is flagged or expunged when the copy affected nothing.
- **neither**: an error naming both extensions. A plain `EXPUNGE` would also remove every other `\Deleted` message in the source mailbox, so the move is refused rather than approximated.

This is not emulating an operation the backend cannot model: copy, flag and expunge are how IMAP expresses a move without the extension, and the adapter already owns all three. `ImapClient` gains `supports_move()` next to `supports_uidplus()`.

## What this is not

It does not make the fallback atomic. Between the copy and the expunge, a concurrent client sees the message in both mailboxes, which is what RFC 6851 exists to avoid and why `MOVE` stays the first choice.
