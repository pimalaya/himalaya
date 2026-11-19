---
cairn: change
id: receipt-rows-carry-their-handle
status: landed
created: 2026-11-19
---

# Receipt rows carry their handle

Owner ruling of 2026-11-19: accepted as recommended, implemented the same day.

## Why

Since pimdir-follows-draft-14 (2026-11-18, unreleased), a pimdir write prints its queue row as a handle string: `{"row":"e2646df031cd876334410c4df14ebd86.1.12","messageId":"d1@x.org"}` (`pimdir message add|send`, `copyRow`, `pimdir mailbox create|rename|delete`, `pimdir flag add|set|remove`, `pimdir tag`), and `pimdir queue show|cancel` echo `row` as given, a string. Three readings of one key now coexist:

- Himalaya's own `pimdir queue list` prints `row` as an integer (src/pimdir/queue/list.rs:93), as pimalaya-cli's output.md requires ("`row` SHALL be an integer, the queue row id `pimdir queue show` and `pimdir queue cancel` take", *A queue row keeps one shape*). So within Himalaya, `row` is a string in a receipt and an integer in the listing of the same row.
- Calendula prints `row` as an integer and `handle` as a string beside it, in its intent receipts (calendula/src/pimdir/intent.rs:174-178, `pimdir reply` and `pimdir cancel`) and, since the-listed-rows-carry-their-handle (2026-11-18), in `pimdir queue list` and `pimdir status`; cardamum's listing does the same (cardamum/cairn/log/2026-11-18-the-listed-rows-carry-their-handle.md).
- Calendula's log of that change notes that `handle` is a key added to the family's queue row shape, Himalaya and cardamum printing none at the time.

A script reading a receipt then cancelling from `queue list` meets one key under two JSON types, and a script written against calendula reads `row` as a number.

## Recommendation

Follow the twins: `row` an integer everywhere (the store's row id), `handle` a string beside it (`<store id>.<incarnation>.<row>`, pimdir STORAGE §4.5), in every receipt (`copyRow` with a `copyHandle`), in `pimdir queue list` rows and in `pimdir queue show|cancel`, which keep taking either form. It is breaking only against master: the string `row` is unreleased (CHANGELOG [Unreleased], Changed, first line), so v3 can ship the aligned shape at no cost, with the MIGRATION line rewritten. pimalaya-cli's output.md should then name `handle` in the shared queue row, so the three apps print one shape.

The other option, `row` a handle string everywhere, breaks the family rule in output.md and both twins.

## Evidence

- himalaya/cairn/spec/backends.md, the `pimdir message add` requirement: `{"row": handle, "messageId": string}`.
- himalaya/src/pimdir/queue/list.rs:93 (`pub row: i64`).
- calendula/src/pimdir/intent.rs:174 (`pub row: i64`), :178 (`pub handle: String`).
- cli/cairn/spec/output.md:78.

## Ruling

The owner accepted the recommendation on 2026-11-19: `row` an integer everywhere, `handle` a string beside it, in every receipt, `pimdir queue list` and `pimdir queue show|cancel`, the JSON Schema included.
