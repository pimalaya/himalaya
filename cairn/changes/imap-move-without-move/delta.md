---
cairn: delta
change: imap-move-without-move
---

# Delta

## ADDED Requirements

### Requirement: IMAP move without the MOVE extension
The IMAP adapter's `move_messages` SHALL use `UID MOVE` (RFC 6851) when the server advertises `MOVE`. Otherwise it SHALL `UID COPY` the set to the target, then flag `\Deleted` in the source. When the copy returns `COPYUID`, those source UIDs SHALL be the ones flagged and, when the server advertises `UIDPLUS` (RFC 4315), `UID EXPUNGE`d. Without `UIDPLUS` it SHALL skip the expunge, leave the messages flagged in the source, and log that at debug. Nothing is flagged or expunged when the copy affected nothing. A plain `EXPUNGE` SHALL NOT be issued, so unrelated `\Deleted` messages stay.
