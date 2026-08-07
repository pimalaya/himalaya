---
cairn: log
change: pimdir-move-supplies-placeholder
date: 2026-08-02
---

# pimdir move supplies a target placeholder

Following the io-replica `staged-delete-and-move` fix (a client-staged `Remove`
or `Move` was a silent no-op in the hub), `ReplicaMutation::Move` gained a
`placeholder` field mirroring `Copy`. The pimdir backend's `move_messages` now
supplies it (`move:<target>:<handle>`), so a move stages a target create plus a
source tombstone that the next sync propagates.

This is a mechanical caller update tracking the engine API; no `backends`
requirement wording changed (the spec already states move stages a `Move`
mutation the sync propagates). Behaviour fixed for users: `message move` — and
`message delete`'s trash-first move step, once `mailbox.alias.trash` is set — now
take effect against a synced pimdir store instead of silently doing nothing.
Verified live over the Fastmail-synced store (`--account local`): a move leaves
the source and appears in the target, keeping the message's public id.

Capability touched: `backends` (no requirement change; caller-side fix logged for
honesty).
