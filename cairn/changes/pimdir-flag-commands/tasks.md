---
cairn: tasks
change: pimdir-flag-commands
---

# Tasks

- [x] `src/pimdir/flag/{cli,list,add,set,remove}.rs`, one module per subcommand, wired into `PimdirCommand`, arguments mirroring `m2dir flag`, `-m` resolved as in the shared flag commands.
- [x] Validate each `-f` before opening a producer: canonical spelling for IANA flags, `\Deleted` and `\Recent` refused, no empty, whitespace or control characters.
- [x] `stage_flags` returns the queue row ids, checking every message before the first enqueue; the shared `store_flags` goes through it.
- [x] Output types `PimdirFlagList` (list) and `PimdirFlagsStaged` (writes; `PimdirFlags` is io-pimdir's), registered in `json_schema.rs` as `himalaya-pimdir-flag-list|add|set|remove`.
- [x] Tests on a temporary store: a keyword round-trips through `add` and `list`; `remove` drops only the named flag; `set` replaces; a flag held in another spelling (`\seen`, `Junk`) is matched as that flag; an unread set lists as `null`; `seen`, `\Deleted`, a backslash word, an IMAP special and an invalid value are refused; a `none` source refuses naming the capability, the source and its detail with nothing queued, a partial one queues; a multi-id call refused part-way queues nothing, and an unknown id is skipped and counted out; a write matching no id fails; the command's arguments pass clap's own check.
- [x] `nix develop --command cargo build/test --bins`; build without `pimdir`; `cargo fmt`; clippy clean.
- [x] CHANGELOG Unreleased entries: the new `pimdir flag` commands, and the shared flag commands on pimdir no longer staging part of a refused multi-id call; fold `delta.md` into `cairn/spec/backends.md`; add the `cairn/log` entry; mark the change `landed`.
