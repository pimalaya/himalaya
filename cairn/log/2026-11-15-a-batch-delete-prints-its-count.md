---
cairn: log
change: a-batch-delete-prints-its-count
landed: 2026-11-15
---

# A batch delete prints its count

Task H11 of the family's cli-alignment-2026-11-15 plan (finding F7). No behaviour changed.

## What landed

- cairn/spec/commands.md states `message delete`'s receipt, `{action, count, queued, errors}`, as the receipt of a batch delete: it takes several ids and counts them, as `message copy` and `message move` do, where Cardamum's and Calendula's deletes of one item print `{id, queued}`.
- pimalaya-cli's cairn/spec/output.md allows it: `count`, `queued` and `errors` keep its meanings, `action` is a key of Himalaya's own the receipt rule does not exclude, and a delete acting on nothing names its targets through one `errors` entry per id.

## Capabilities moved

- [commands](../spec/commands.md): *A write prints what it acted on*.
