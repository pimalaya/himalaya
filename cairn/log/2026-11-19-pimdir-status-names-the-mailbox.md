---
cairn: log
change: pimdir-status-names-the-mailbox
landed: 2026-11-19
---

# pimdir status names the mailbox

The owner's ruling of 2026-11-19: `himalaya pimdir status` names a parked or refused row's mailbox `mailboxId`, as Calendula's and Cardamum's name theirs `calendarId` and `addressbookId`, where it printed `collection`, a pimdir word. The command is unreleased (it landed the same day under pimdir-status-reports-the-store), so no user is broken and no CHANGELOG entry moves.

## What landed

- src/pimdir/status.rs: `PimdirParkedStatus.mailbox_id`, and a status-own `PimdirRefusedStatus` (`mailboxId`, `itemId`, `source`, `reason`, `refusedAt`), so `pimdir queue list` keeps its `collection` key.
- src/pimdir/backend.rs: `status` maps the queue's refused rows onto it.
- cairn/spec/backends.md: *pimdir status reports the store*, its row shapes and scenario.

## Checks

The test `a_status_prints_mailboxes_parked_rows_and_refusals` asserts the new keys, and the store test with a parked move reads `mailbox_id`.

## Left

`pimdir queue list`, `queue show` and their refusals still print `collection`, as Calendula's and Cardamum's queue outputs do: aligning the queue outputs is a family question, not this change's.
