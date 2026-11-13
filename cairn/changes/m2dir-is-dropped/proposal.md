---
cairn: change
id: m2dir-is-dropped
status: landed
created: 2026-11-13
---

# m2dir is dropped

## Why

The owner decided on 2026-11-13 that io-m2dir leaves the Pimalaya family: pimdir supersedes m2dir as the local store the family syncs into, and io-m2dir will see no further work. Himalaya's m2dir backend was never on by default, lacked mailbox `rename`, and would keep a dead library in the dependency graph and a backend in every matrix, capability table and spec.

## What

- Delete src/m2dir/, the `m2dir` cargo feature and the io-m2dir dependency, the `BackendClient::M2dir` arms, the `m2dir` value of `--backend`, the m2dir capability table, the `m2dir` account check, the m2dir JSON Schemas and every `feature = "m2dir"` of the cfg lists.
- An `m2dir` block in a configuration keeps loading, ignored whatever it holds, as the v1 `backend` table does: the account section tolerates unknown keys for himalaya-tui, so a hard error would break a file the TUI shares. When no configured backend matches, the error names the `m2dir` block as ignored since v3 and points at MIGRATION.md. `-b m2dir` is refused by clap as an unknown value.
- config.sample.toml loses its m2dir section; the CI clippy matrix and the build.rs `backend` cfg lose m2dir.
- MIGRATION.md tells an m2dir user to move to Maildir or pimdir; CHANGELOG gets one BREAKING Removed entry, the Unreleased entries about m2dir dropped or trimmed; README and CONTRIBUTING stop listing it.
- The spec stops naming m2dir; the cairn/spec/testing/m2dir.md and m2dir-specific.md reports go, with their row in cairn/spec/testing/README.md.

## Scope / non-goals

No converter from m2dir to Maildir or pimdir: a user re-syncs from the server. The logs and the older changes naming m2dir stay as history, as does the dated cairn/spec/testing/maildir.md report.
