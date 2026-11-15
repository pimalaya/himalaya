---
cairn: change
id: the-queue-row-is-shared
status: landed
created: 2026-11-15
---

# The queue row is shared

## Why

Rulings D19 and D21 of the family's cli-alignment-2026-11-15 plan (report X3, row M9, and the queue part of schema review A4), now pimalaya-cli's cairn/spec/output.md (*A queue row keeps one shape*): every `pimdir queue list` row, pending or parked, prints `{row, kind, collection, linkId, itemId, producer, queuedAt, attempts, unconfirmed, error}` plus the domain's label, `subject` for mail, every key printed. Himalaya printed a pending row as `{row, queuedAt, producer, send, envelope}`, with no `kind`, `collection` nor `linkId`, and a parked row as `{row, queuedAt, producer, kind, attempts, unconfirmed, error}`.

## What

- One row type, `PimdirQueueRow`, for `messages` (pending) and `parked` alike: `row`; `kind` as io-pimdir holds it (`PimdirAction::kind`, `PimdirParkedAction::action`); `collection` the mailbox's id; `linkId` the bare `Message-ID` an `add` creates, `null` otherwise, a `submit` among them (the unified send prints a `null` id); `itemId` the public id of the message a `set-flags`, `move`, `copy`, `remove` or `update` acts on; `producer`, `queuedAt`, `attempts`, `unconfirmed`; `error` `null` while pending; `subject` from the body the row pins, else from the message it acts on, a parked `submit` (whose row names no body) giving its payload's subject, `null` when there is none to read.
- A parked row's payload is decoded through io-pimdir's `action_from_payload`; one that does not decode addresses nothing.
- `send` (now `kind` `submit`) and `envelope` (its subject kept as `subject`) are dropped (D21).
- Text: the row, its state, when it was queued, its kind, then the subject (else the message id) of a queued row or why a row was parked; the staged flags and the recipients no longer print.
- Which rows list is unchanged: the mailbox's pending `add` and `submit` rows, and its parked rows of any kind.
- No MCP tool prints the queue.
