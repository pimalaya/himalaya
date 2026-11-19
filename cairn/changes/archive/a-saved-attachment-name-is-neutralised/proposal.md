---
cairn: change
id: a-saved-attachment-name-is-neutralised
status: landed
created: 2026-11-19
---

# A saved attachment name is neutralised

## Why

Line C3 of pimdir's audit-2026-11-12 (pimdir/cairn/changes/audit-2026-11-12/tasks.md): `attachment download` saved a part under the name its message gave, keeping control and bidirectional formatting characters, which pimdir STORAGE §18 forbids in a name that becomes a file. A right-to-left override shows `invoice<U+202E>fdp.exe` as `invoiceexe.pdf` in a file manager.

## What

The download name becomes one path component by §18's rule: path separators, control characters and the twelve bidirectional formatting characters §18 names replaced by `_`, on every backend.

io-pimdir 0.17.0 has the exact rule public (`security::path_component`), but io-pimdir is an optional dependency of Himalaya (the `pimdir` feature), and the download serves every backend, so the rule is written in src/shared/attachment/download.rs. The audit line's own direction, the rule in pimalaya-cli's `sanitize` module, would let every frontend share it: pimalaya-cli 0.6.1 has no such function yet.
