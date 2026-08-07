---
cairn: change
id: pimdir-cache-backend
status: landed
created: 2026-08-01
---

# pimdir cache backend (read + staged writes)

## Why

Himalaya can read/write remote mailboxes and local file stores (Maildir, m2dir),
but not a **pimdir** store — the SQLite-indexed, content-addressed local cache the
sync engine (Neverest / io-replica + io-pimdir) populates. Reading the same store
the sync writes gives an indexed, offline, provider-agnostic mailbox with no second
copy and no format bridge (LOCAL_STORE_PLAN §4 / action plan M4–M5).

pimdir is a **cache, not a live backend**: an item may be un- or partially
hydrated. The store surfaces that (`PimdirItem.level` + absent `object`); Himalaya
owns the UI reaction, so a not-downloaded message reads as "body not fetched"
rather than an error.

## What

A new `pimdir` feature and `src/pimdir/` (`client.rs` + `backend.rs` + `hash.rs`),
a `BackendClient::Pimdir` variant, and the local-before-network `select_storage`
arm — mirroring m2dir.

Reads (source-independent; observe the shared items via io-pimdir's client read
API): `list_mailboxes` (mail collections), `list_envelopes` and `search_envelopes`
(built from the `v: 1` meta, no body reads), `get_message` (blob read, or a clear
"body not fetched" when `level < Full`).

Writes (staged io-replica mutations a later sync pushes, never raw SQL):
`store_flags`→`SetFlags`, `add_message`→`Add` (content-hash matches Neverest for
dedup), `copy_messages`→`Copy`, `move_messages`→`Move`, `delete_messages`→`Remove`.
A write is attributed to the configured `pimdir.source`; on a store never synced as
that source (no binding) it fails loudly rather than staging a change no sync
carries.

## Scope / non-goals

- **Cache semantics, not a live backend**: no fetch-on-demand (a pure reader has no
  remote — hydration is the sync's job); no native trash; body search only on
  hydrated items.
- **Shared subcommands only** — no dedicated `himalaya pimdir …` native subcommands
  yet (no `cli.rs`), so no `json_schema.rs` entries.
- **Source alignment is a deployment concern** (single-writer + source identity =
  action plan M6); documented, guarded, not solved here.
- Depends on the unpublished io-pimdir/io-replica via path deps (all local WIP).
