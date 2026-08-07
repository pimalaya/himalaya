---
cairn: log
change: pimdir-cache-backend
landed: 2026-08-01
---

# pimdir cache backend (read + staged writes)

Added a `pimdir` storage backend: Himalaya over a local pimdir store (io-pimdir +
io-replica) used as the offline cache the sync engine populates (action plan
M4+M5). Mirrors the m2dir wiring — a `pimdir` feature, `src/pimdir/`
(`client.rs`/`backend.rs`/`hash.rs`), a `BackendClient::Pimdir` variant, the
local-before-network `select_storage` arm, `Backend::Pimdir`, and a
`PimdirConfig { root, source }`.

Reads are source-independent (io-pimdir's client read API over the shared items)
and built from the stored `v: 1` meta without body reads: `list_mailboxes`,
`list_envelopes`, `search_envelopes` (in-memory sort/filter/paginate like the file
backends; body clauses only match hydrated items), and `get_message`. Availability
is surfaced honestly: `get_message` on an item with `level < Full` (no local
`object`) returns a clear "body not fetched" rather than an error — the item still
lists; the UI reaction is Himalaya's, the store only reports `level`.

Writes go through the io-replica `mutate` seam (never raw SQL) so the next sync
derives and pushes them: `store_flags`→`SetFlags`, `add_message`→`Add`,
`copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`.
`add_message` content-hashes with the same 128-bit FNV digest as Neverest
(`src/pimdir/hash.rs`) so an added message dedups against a synced one. Each write
is attributed to the configured `pimdir.source` and guarded: on a store never
synced as that source (the placement has no base binding) it fails loudly rather
than staging a change no sync would carry — the honest handling of the
source-identity / single-writer question (action plan M6), documented not solved.

Depends on the unpublished io-pimdir/io-replica via path deps (all local WIP).

Verified: `cargo build`/`test --bins` green (76 tests, incl. four new pimdir unit
tests — envelope-from-meta, add link/meta derivation, flag-op algebra, content-hash
shape), fmt + clippy clean.

Spec updated: `backends` (MODIFIED: local storage backends now include pimdir;
ADDED: pimdir is an availability-aware cache).
