---
cairn: log
change: m2dir-is-dropped
landed: 2026-11-13
---

# m2dir is dropped

io-m2dir left the Pimalaya family on the owner's decision of 2026-11-13, pimdir superseding m2dir, and Himalaya dropped its m2dir backend: src/m2dir/, the `m2dir` cargo feature, the io-m2dir dependency, `-b m2dir`, the `himalaya m2dir` commands and their JSON Schemas, the m2dir capability table, its `account check` row and its section of config.sample.toml. The CI clippy matrix and the build.rs `backend` cfg lost m2dir.

An `m2dir` block still loads, ignored whatever it holds, following the v1 `backend` table: the account section tolerates unknown keys for himalaya-tui, so refusing the block would break a shared file. An account left with no other backend fails naming the ignored block and MIGRATION.md, which tells an m2dir user to move to a Maildir or a pimdir store.

Capabilities moved: config ("An m2dir block is named, not loaded" added; "Multi-account schema", "Path keys expand as the configuration is read", "One sample configuration, with a section per backend" modified), backends ("Local storage backends", "Sending transport", "Mailbox role", "Mailbox counts are asked for", "Every backend builds alone", "The envelope carries its threading pointers", "Backends reading the MIME structure mark attachments by the pimdir rule"), commands ("Unified commands over a resolved transport", "account check covers every declared backend", "An account list names every block", "Text output is plain text", "A command given message ids says how many it found", "Unified mailbox writes", "A write prints what it acted on"), packaging ("Binary only", "Feature-gated backends"), search ("Per-backend translation"). The m2dir test reports under cairn/spec/testing/ went with their row; the Unreleased CHANGELOG entries about m2dir were dropped or trimmed, one BREAKING Removed entry replacing them.
