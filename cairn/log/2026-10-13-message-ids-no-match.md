---
cairn: log
change: message-ids-no-match
landed: 2026-10-13
---

# A command given message ids fails when none of them exists

Reported in [#777]. Over IMAP the flag commands reported a success for an id matching nothing, and `message move` and `message copy`, which count what they moved or copied since [#781], still exited 0 when that was nothing. `store_flags` now returns a count on every backend, and `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` (to the trash) fail when it is zero. The flag commands report the count, and their plain output, with that of `message delete`, ends with a newline.

Only IMAP needed counting, the other backends failing on an unknown id already. It reads the `UID STORE` echoes and asks a `UID SEARCH UID <set>` when they are fewer than the UIDs given.

Verified against Gmail over IMAP: an unknown id failed `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` with exit code 1, under `--json` too; a known id succeeded, also when its flags did not change; a known and an unknown id together reported one message. The backends other than IMAP were not exercised beyond the test suite.

Capabilities moved: **commands** (A command given message ids says how many it found), **backends** (IMAP flag counts).

[#777]: https://github.com/pimalaya/himalaya/issues/777
[#781]: https://github.com/pimalaya/himalaya/issues/781
