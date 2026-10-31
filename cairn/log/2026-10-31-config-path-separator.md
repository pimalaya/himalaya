---
cairn: log
change: config-path-separator
landed: 2026-10-31
---

# Took the config path separator from the platform

A Windows absolute path carries a colon of its own. `ConfigPathsArg` declared
`value_delimiter = ':'`, so clap split every `-c` value at the drive letter:
`-c C:\Users\me\config.toml` reached the loader as `C` and
`\Users\me\config.toml`, neither of which exists, and the configuration was
never read even though the file was right there. `HIMALAYA_CONFIG` feeds the
same argument and failed the same way, and so did repeated `-c` flags with
absolute paths. A relative path worked, which is why it stayed hidden.

The delimiter now follows `$PATH` on the platform: `;` on Windows and `:` on
every other target. A single path is never split, the help text says which
separator applies, and several paths can still be given at once or by
repeating `-c`. On Windows a colon-joined list is no longer split, and a path
that itself contains `;` now is; both are the price of reading absolute paths
whole, and the documented spelling on Windows is `;` or repeated flags.

Verified: `cargo build`, `cargo fmt --check` and `cargo test --all-features`
run on Windows before and after. The failures already present there, the
pimdir, mbox and tilde-expansion ones, are unchanged by name; one integration
test passing `-c` an absolute path now passes instead of failing on the
missing configuration. Two new tests parse the CLI: an absolute path yields one
path (before the change it yielded `["C", "\Users\...\config.toml"]`), and two
paths joined with the platform separator yield two. A third pins the constant
to `;` on Windows and `:` elsewhere, so dropping the platform gate cannot pass
unnoticed.

Spec updated: config (MODIFIED: Config loading and merge).
