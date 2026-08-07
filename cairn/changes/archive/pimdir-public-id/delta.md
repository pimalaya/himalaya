---
cairn: change
change: pimdir-public-id
---

## ADDED Requirements

### Requirement: pimdir shows a short public id
The pimdir backend SHALL show and accept each message's per-collection public id
(`items.seq`, a small integer) as its `Envelope.id`, not the internal `link_id`.
It SHALL resolve the id to the `link_id` (via the store's `get_item` /
`seq_for_link`) before reading a body or staging an edit, and SHALL fail clearly
on a non-numeric id. `add_message` SHALL return the new item's `seq`.
