---
cairn: log
change: pimdir-public-id
landed: 2026-08-02
---

# The pimdir backend shows a short public id, not the link id

The pimdir backend set `Envelope.id = link_id` (a long `mid:…@…` string) that
filled the envelope table's ID column and forced users to handle it. io-pimdir now
assigns each item a per-collection public `seq`; the backend uses it:

- `envelope_from_item` sets `id = seq.to_string()`.
- `get_message` and `synced_placement` (the write ops' resolver) parse the id as a
  `seq` and resolve it to the internal `link_id` via `store.get_item(collection,
  seq)` before reading the body / staging the mutation. `parse_id` fails clearly
  on a non-numeric id.
- `add_message` returns the new item's `seq` via `store.seq_for_link`.

Verified against a live neverest-synced Stalwart store: `envelope list` shows ids
1..N (fitting the table), `message read 3` reads by the short id, `flag add 2
--flag flagged` resolves the id and stages the mutation, and a non-numeric id
gives "Invalid message id … (expected a number)".

Spec updated: `backends` (ADDED "pimdir shows a short public id").
