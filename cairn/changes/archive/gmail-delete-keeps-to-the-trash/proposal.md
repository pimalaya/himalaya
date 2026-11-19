---
cairn: change
id: gmail-delete-keeps-to-the-trash
status: abandoned
created: 2026-11-19
---

# Gmail delete keeps to the trash

Owner ruling of 2026-11-19: kept as is, no code; closed as abandoned.

## Why

The question: Himalaya's Gmail removal calls `users.messages.delete`, permanent, while neverest removes the label.

## What the code does

- Himalaya's unified `message delete` never deletes outside the trash. It resolves the trash (`mailbox.alias.trash`, else the `TRASH` role) and, from any other mailbox, moves the messages there (src/shared/message/delete.rs:57-85): on Gmail one `messages.modify` adding `TRASH` and removing the source label (src/gmail/backend.rs, `move_messages`), the other labels kept. Only from the trash does it call `delete_messages`, `messages.delete` through batches (src/gmail/backend.rs:401-417), printing `"action":"deleted"`.
- The live run of 2026-11-15 checked both paths on the Google test account: "from a label (to TRASH), from TRASH (permanent) ... pass (`moved-to-trash`, then `deleted`)" (.github/cairn/changes/live-testing-2026-11-15/reports/himalaya/gmail-google.md:26).
- neverest/cairn/spec/sync.md:1582: "A delete from a label SHALL remove that label (from `INBOX` it archives), a move SHALL swap two labels in one modify, and only a delete from `TRASH` SHALL delete permanently."

So both delete permanently from `TRASH` alone. They differ outside it: neverest replays a store removal (an item gone from one collection), which on a label means unlabelling, the message staying in All Mail; Himalaya's `message delete` is the user's delete, which on every backend means the trash, then for good from the trash.

## Recommendation

Keep Himalaya as it is: one meaning of `message delete` across backends (commands.md, *A write prints what it acted on*: `moved-to-trash`, `deleted` from the trash), the permanent step only from `TRASH`, as neverest. Removing a label alone stays reachable as `message move` to another label, or `gmail messages modify --remove-label`. If unlabelling is wanted as a unified verb, it is a new verb (a `message remove`, for the label backends and JMAP's `mailboxIds`), not a new meaning of `delete`.

## Ruling

The owner ruled on 2026-11-19 to keep Himalaya as it is: `message delete` deletes for good only from `TRASH` and moves to `TRASH` from any other label, on Gmail as on every backend. Removing a label alone would be a new verb, not a new meaning of `delete`, and none is planned. The spec already states the behaviour (commands.md, *A write prints what it acted on*), so nothing changes and the change closes as abandoned.
