---
cairn: change
id: pimdir-queue-row
status: landed
created: 2026-10-04
---

# pimdir writes name their queue row

## Why

A staged draft or send has no id until the sync engine applies it. MOA follows one by scanning `pimdir queue list` for its `Message-ID`, to cancel a send and to know when a draft is stored. The row id is known the moment the row is enqueued; the shared outputs dropped it on purpose (shared-output-free-of-pimdir), so the `pimdir` namespace is where it belongs.

## What

- `pimdir message add` and `pimdir message send`, the shared add and send staged the same way, print `queueId` and `messageId` (and the copy's fate for a send).
- `pimdir queue show <ROW>`: `pending`, `parked` with why, or `gone`.
- The backend's `add_message` and `send_message` return the row and the link id; the shared client keeps returning what it did.

Left for io-pimdir: once the owner applies a row it is deleted, with no record of the item it produced, so `gone` cannot yet tell applied from cancelled nor give the applied `seq`. When io-pimdir records "row → applied seq", `queue show` gains `applied` with `collection` and `seq`.
