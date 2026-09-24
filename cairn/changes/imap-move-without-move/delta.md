---
cairn: delta
change: imap-move-without-move
---

# Delta

## ADDED Requirements

### Requirement: IMAP move without the MOVE extension
The IMAP adapter's `move_messages` SHALL use `UID MOVE` (RFC 6851) when the server advertises `MOVE`. Otherwise, when it advertises `UIDPLUS` (RFC 4315), it SHALL `UID COPY` the set to the target, then flag the same UIDs `\Deleted` in the source and `UID EXPUNGE` exactly those UIDs, skipping both when the copy affected nothing. With neither extension it SHALL fail, naming both, rather than issue a plain `EXPUNGE` that would remove unrelated `\Deleted` messages.
