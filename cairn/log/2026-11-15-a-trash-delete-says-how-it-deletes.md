---
cairn: log
change: a-trash-delete-says-how-it-deletes
landed: 2026-11-15
---

# A trash delete says how it deletes

[#814](https://github.com/pimalaya/himalaya/issues/814): `message delete -m Trash 999999` on Dovecot 2.4.5 (UIDPLUS) printed "Flagged 0 message(s) as deleted; run an expunge on the trash to remove them", `"action":"flagged","count":0`. The IMAP adapter reported a removal only when it found a UID (`outcome.value > 0 && supports_uidplus()`), so finding none read as flagged, and the advice would have expunged every `\Deleted` message of the trash, other clients' included.

## What landed

- src/imap/backend.rs: `delete_messages` reports whether the trash removes from `supports_uidplus()` alone; only the `UID EXPUNGE` waits for a found UID. `a_trash_delete_finding_nothing_still_says_how_it_deletes`.
- src/shared/message/delete.rs: a `flagged` result with `count` 0 prints "Flagged no message as deleted", the expunge advice kept for a count above 0. `a_flag_delete_advises_an_expunge_only_when_it_flagged`.
- Both tests fail against the code before the fix.

## Capabilities moved

- [backends](../spec/backends.md): *An IMAP trash delete says how the trash deletes* added.
