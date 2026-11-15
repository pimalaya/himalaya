---
cairn: change
id: a-batch-delete-prints-its-count
status: landed
created: 2026-11-15
---

# A batch delete prints its count

## Why

Task H11 of the family's cli-alignment-2026-11-15 plan (finding F7). `message delete` prints `{action, count, queued, errors}` where Cardamum's and Calendula's deletes print `{id, queued}`. The two differ because Himalaya's delete takes several ids, as `message copy` and `message move` do, and the twins' delete one item; the spec named the delete's `action` beside `count` but no receipt shape.

## What

No behaviour changes. cairn/spec/commands.md states the receipt of a batch delete, `{action, count, queued, errors}`, beside the receipts of the other unified writes, and why it counts rather than names. pimalaya-cli's cairn/spec/output.md allows it: `count`, `queued` and `errors` carry the meanings it gives them, and its list of receipt keys fixes meanings without closing the set, so `action` is Himalaya's own; a delete that acted on nothing still names its targets, one `errors` entry per id.
