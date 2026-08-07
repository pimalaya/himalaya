---
cairn: tasks
change: pimdir-public-id
---

- [x] `envelope_from_item` sets `id = seq`.
- [x] `get_message` / `synced_placement` parse the id as `seq`, resolve to link id.
- [x] `add_message` returns the new `seq`; `parse_id` errors clearly on non-numeric.
- [x] Build + fmt clean.
- [x] Verified against a live neverest store: short ids 1..N in the table,
      read/flag by id, clear error on a non-numeric id.
- [ ] Fold delta into `cairn/spec/backends.md`; write log entry.
