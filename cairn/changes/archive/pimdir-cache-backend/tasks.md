---
cairn: tasks
change: pimdir-cache-backend
---

# Tasks

- [x] Cargo `pimdir` feature + path deps (io-pimdir/client, io-replica); `build.rs`
      backend list; `default` features.
- [x] `Backend::Pimdir` + `allows_pimdir` + Display.
- [x] `config`: `PimdirConfig { root, source }` + `AccountConfig.pimdir`.
- [x] `src/pimdir/`: `client.rs` (open store as source + blobs), `hash.rs`
      (Neverest-matching content hash), `backend.rs` (read + write adapter).
- [x] `shared/client.rs`: import, `BackendClient::Pimdir`, all dispatch arms,
      `select_storage` arm (local-before-network).
- [x] Reads from meta (no body reads); `get_message` "body not fetched" when
      `level < Full`; writes via the `mutate` seam with a synced-source guard.
- [x] Tests: envelope-from-meta, add link/meta derivation, flag-op algebra,
      content-hash shape.
- [x] `nix develop --command cargo build/test --bins`; `cargo fmt`; clippy clean.
- [x] Fold `delta.md` into `cairn/spec/backends.md`; add `cairn/log` entry;
      mark change `landed`.
