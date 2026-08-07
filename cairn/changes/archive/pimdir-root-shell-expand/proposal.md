---
cairn: change
id: pimdir-root-shell-expand
status: landed
created: 2026-08-02
---

# Shell-expand `pimdir.root`

## Why

`pimdir.root` is a `PathBuf` deserialized verbatim, with no `~`/env-var
expansion (unlike the SASL string fields). A user pointing it at a Neverest store
with the natural `pimdir.root = "~/.local/state/neverest/<account>"` had every
read come up empty: himalaya opened the *literal* relative path `./~/.local/…`,
silently **creating a new empty store** there (`PimdirStore::open` creates on
absence), so `mailbox list` — and everything downstream — was empty, with no
error. (It also littered a `~` directory in the working directory.)

## What

Expand `~` and env vars on `pimdir.root` in `PimdirClient::new` before opening
the store and the blob reader, via `shellexpand::full` (the crate already used by
the wizard), falling back to the raw path if expansion fails. A store path written
with `~` now resolves to the home-relative store.

Verified against a real Neverest-synced store (read-only): `mailbox list` returns
the 16 synced mailboxes, `envelope list` renders from the `v:1` meta, and
`message read` pulls the body from the blob.
