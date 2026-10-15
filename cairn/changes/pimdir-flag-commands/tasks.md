---
cairn: tasks
change: pimdir-flag-commands
---

# Tasks

- [ ] `src/pimdir/flag/{cli,list,add,set,remove}.rs`, one module per subcommand, wired into `PimdirCommand`, arguments mirroring `m2dir flag`, `-m` resolved as in `pimdir message add`.
- [ ] Validate each `-f` before opening a producer: canonical spelling for IANA flags, `\Deleted` and `\Recent` refused, no empty, whitespace or control characters.
- [ ] `store_flags` returns the queue row ids, and checks every message before the first enqueue.
- [ ] Output types `PimdirFlagList` (list) and `PimdirFlagsStaged` (writes; `PimdirFlags` is io-pimdir's), registered in `json_schema.rs` as `himalaya-pimdir-flag-list|add|set|remove`.
- [ ] Tests on a temporary store: a keyword round-trips through `add` and `list`; `remove` drops only the named flag; `set` replaces; `seen`, `\Deleted` and an invalid value are refused; a `none` source refuses with nothing queued, a partial one queues with the warning; a multi-id call refused on its last id queues nothing.
- [ ] Manual run against a Gmail source, recorded under `cairn/spec/testing/`.
- [ ] `nix develop --command cargo build/test --bins`; build without `pimdir`; `cargo fmt`; clippy clean.
- [ ] CHANGELOG Unreleased entry; fold `delta.md` into `cairn/spec/backends.md`; add the `cairn/log` entry; mark the change `landed`.
