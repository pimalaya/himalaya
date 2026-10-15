---
cairn: change
id: message-ids-no-match
status: landed
created: 2026-10-13
---

# A command given message ids fails when none of them exists

## Why

Reported in [#777]. Over IMAP, `flag add`, `flag set` and `flag remove` print `Successfully …` and exit 0 for an id that matches nothing, since `UID STORE` answers `OK` either way and `store_flags` returns `()`. `message move` and `message copy` count what they moved or copied since the fix for [#781], but on zero they print `No message moved: no id matched in the source mailbox` and still exit 0. Neither a person nor a script can tell that nothing happened.

The other backends already fail on an unknown id (JMAP `notUpdated`, an HTTP 404 on Gmail and Graph, a lookup error on Maildir, m2dir, mbox and pimdir), so the gap is IMAP's, and the count is what closes it.

## What

- `store_flags` returns how many of the given messages were found, on every backend, as `move_messages` and `copy_messages` do.
- `flag add`, `flag set`, `flag remove`, `message move`, `message copy` and `message delete` (when it moves to the trash) fail, exit code 1, when the count is zero. When only some ids match they succeed and report the count.
- The flag commands report that count: `Successfully added flags to 2 message(s): seen`, and `count` beside `flags` under `--json`.
- Over IMAP the untagged `FETCH` echoes of the `UID STORE` are the count; fewer echoes than UIDs (a server may skip a message whose flags did not change) is settled by a `UID SEARCH UID <set>`, the same search the copy and move counts of [#781] use.
- The plain output of the flag commands and of `message delete` ends with a newline, as the other commands' does.

## Out of scope

- `message delete` on a message already in the trash: `delete_messages` returns whether the messages were expunged, not how many, so that branch still cannot tell an unknown id.
- A stable `--json` error `code` for "no id matched". It fits `json-error-code` and can follow if wanted.
- The newline missing after piped `--json` output, which is the printer's in pimalaya/cli and applies to every command.

[#777]: https://github.com/pimalaya/himalaya/issues/777
[#781]: https://github.com/pimalaya/himalaya/issues/781
