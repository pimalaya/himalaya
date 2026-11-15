---
cairn: change
id: pimdir-follows-draft-10
status: landed
created: 2026-11-13
---

# The pimdir backend follows storage-draft-10, sync-draft-10 and search-draft-10

## Why

io-pimdir 0.11.0 (crates.io, tag v0.11.0) follows pimdir ae8b231: storage-draft-10, sync-draft-10 and search-draft-10, tagged on 2026-11-13. neverest writes draft-10 stores from 52f83a9, and drafts are exact, so a Himalaya on io-pimdir 0.10 refuses every store it writes. With the pin at 0.11 (46e8b4ea) the `pimdir` feature stopped compiling, on about twenty sites of src/pimdir.

## What

- The compile port: an enqueue answers `PimdirEnqueued { handle, partial }`, `enqueue_with_body_checked` is `enqueue_with_body`, `PimdirAction::SetFlags` takes `base`, `PimdirAction::Remove` takes `if_match` and `resolves`, `PimdirActionStatus` gains `Unconfirmed` and `Reset` and its `Applied` names an optional collection, `PimdirSearchLocal::bodiless` is `kinds`, a tag plan answers handles.
- A queue row stays the integer it was (`handle.id`): `pimdir queue show` and `cancel` take a row id, which io-pimdir follows unchecked. Handles as strings are left to a later change (audit line C4).
- One gate: `producer.check` before `enqueue` goes, the enqueue running the gate of STORAGE §15.6 itself and answering the partial supports. `stage_flags` appends its rows as one batch (`enqueue_batch`), all or none, and names the message the gate refused.
- `set-flags` states its `base` for `flag add`, `flag remove` and the `\Seen` a read stages, so a flag a sync lands while the row waits survives; `flag set` states none and replaces.
- `remove` states neither `if_match` nor `resolves`: Himalaya reads no version before a delete and decides no conflict, so the owner parks a delete over a conflicted message.
- `pimdir queue show` reads `unconfirmed` and `reset`, an applied row of a deleted mailbox reads `applied` with no collection, `pimdir queue list` marks an unconfirmed row, and `pimdir queue cancel` says an unconfirmed row may have taken effect.
- The coverage carries `missing`, `unparseable` and `encrypted`, raw counts, the answer incomplete only while a term reads what they hide, as calendula does. `complete` is io-pimdir's `is_complete`, which counts the index's lag itself.
- `store-reset` and `lock-replaced` join the relayed codes.

## Out of scope

Handles as strings (C4), `queue retry`, a view of conflicts, and the audit lines C3 and C5. The end-to-end run on a scratch store a draft-10 neverest synced, the suites of tests/ being run once every app has moved (owner, 2026-11-13).
