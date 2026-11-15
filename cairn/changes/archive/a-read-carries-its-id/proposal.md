---
cairn: change
id: a-read-carries-its-id
status: landed
created: 2026-11-15
---

# A read carries its id

## Why

Task H8 of the family's cli-alignment-2026-11-15 plan (finding F9, owner ruling D7). pimalaya-cli's cairn/spec/output.md says a read carries the item's `id` and `etag`, `etag` `null` where the backend versions no item; Cardamum's `card read` carries both, Himalaya's `message read --json` carried neither.

## What

`MessageReadOutput` is `{id, etag}` beside the view, flattened: `id` the id the command took, `etag` `null`, a message's body never changing and no write taking a version to match. The view is `MessageReadView`, which `message parse` prints without `id` and `etag`, a parsed file having neither. The text output is unchanged.
