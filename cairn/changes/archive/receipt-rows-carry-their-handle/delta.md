---
cairn: delta
change: receipt-rows-carry-their-handle
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A queue row is named by its handle (backends.md)

Every pimdir output naming a queue row prints `row`, the row id, an integer, and `handle` beside it, the row's `PimdirActionHandle`: the receipts of `pimdir message add|send` (`copyRow` with `copyHandle`), `pimdir mailbox create|rename|delete`, `pimdir flag add|set|remove` and `pimdir tag` (`null` both where nothing was staged), the rows of `pimdir queue list`, and `pimdir queue show|cancel`, whose `handle` is the handle given or the bare id given named under the store's identity, where `row` was the handle string, echoed as given by `queue show|cancel`.

### Requirement: The pimdir subcommand reads and retracts the queue (backends.md)

A `pimdir queue list` row carries `handle` after `row`, read from `PimdirReader::change_cursor`, the text printing it in place of the bare id; `queue cancel` prints `{row, handle, kind, unconfirmed}`.

### Requirement: pimdir writes name their queue row (backends.md)

`{"row": integer, "handle": string, "messageId": string}`, `copyRow` and `copyHandle`, the flag rows `{id, row, handle}`, the mailbox intents `{row, handle, ...}`, `queue show` `{row, handle, state, ...}`.

### Requirement: Text output is plain text (commands.md)

`pimdir tag` and `pimdir queue list` print the row's handle in their row cell.

## REMOVED Requirements

None.
