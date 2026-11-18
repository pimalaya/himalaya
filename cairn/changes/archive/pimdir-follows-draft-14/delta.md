---
cairn: delta
change: pimdir-follows-draft-14
---

# Delta

## ADDED Requirements

### Requirement: A queue row is named by its handle

As folded into cairn/spec/backends.md: a pimdir write's receipt names its row by `PimdirActionHandle` (store id, incarnation, row id joined by periods); `pimdir queue show` and `queue cancel` take a handle, checked, or a bare id, unchecked, echoing `row` as given; a handle of a restored copy reads `reset` and is refused `store-reset` by `queue cancel`, the row left queued.

## MODIFIED Requirements

### Requirement: A pimdir store failure carries its code (backends.md)

A `store-version` refusal is io-pimdir's `PimdirFormatRefusal` naming himalaya: which store is the earlier, an earlier one synced out with its engine then moved aside, a later one asking to upgrade himalaya, never to delete it. The `store-version` row of commands.md says the same.

### Requirement: pimdir writes name their queue row (backends.md)

`row` and `copyRow` of `pimdir message add|send`, `pimdir mailbox create|rename|delete` and `pimdir flag add|set|remove` are a handle string, where they were `i64`.

### Requirement: pimdir searches through the store's index (search.md)

The query language is search-draft-14 §8.

## REMOVED Requirements

None.
