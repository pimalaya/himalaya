---
cairn: change
id: pimdir-public-id
status: landed
created: 2026-08-02
---

# The pimdir backend shows a short public id, not the link id

## Why

The pimdir backend set `Envelope.id = link_id` — the long `mid:…@…` string — so it
filled the envelope-table ID column and forced users to type/paste it. The link
id is an internal key; a user should see a short id like every other backend
(IMAP shows UIDs).

## What

io-pimdir now assigns each item a per-collection public `seq` (small integer,
IMAP-UID-like). The backend shows it and accepts it everywhere:

- `list_envelopes`/`search_envelopes` set `Envelope.id = seq`.
- `get_message` and the write ops (`store_flags`, `copy`/`move`, `delete`) parse
  the id as a `seq` and resolve it to the internal `link_id` (via `get_item` /
  `synced_placement`) before operating; a non-numeric id fails clearly.
- `add_message` returns the new item's `seq` (via `seq_for_link`).
