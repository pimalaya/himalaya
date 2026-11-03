---
cairn: change
id: the-rc-file-layers-over-the-config
status: landed
created: 2026-11-03
---

# The rc file layers over the config

## Why

With both `~/.config/himalaya/config.toml` and `~/.himalayarc` on disk, only the first is read: `TomlConfig::first_valid_default_path` in pimalaya-config stops at the first default path that exists, so the rc file is silently ignored. A user who keeps a shared config and an rc of local overrides (or the reverse) loses half their accounts with no message.

## What

The default paths become layers, each existing one deep-merged on top of the ones before, the same merge `-c a:b` already performs: `$XDG_CONFIG_HOME/<project>/config.toml` (the platform config directory), then `$HOME/.config/<project>/config.toml`, then `$HOME/.<project>rc`. The rc file therefore overrides, as `~/.gitconfig` does over `$XDG_CONFIG_HOME/git/config`.

The alternative of reading the rc file alone when it exists was rejected: it only moves the shadowing to the other file, and flips which config every user holding a forgotten rc runs on.

Details the merge forces:

- Paths are deduplicated by their canonical form before merging. On Linux without `XDG_CONFIG_HOME` the first two are one file, and an rc symlinked to the config is a common setup; merging a file onto itself concatenates every array (`merge_toml` appends arrays), doubling `imap.alpn` and the like.
- `-c` keeps replacing the defaults entirely: an explicit path is no layer.
- Two accounts marked `default = true` across layers become an error. `Config.accounts` is a `HashMap`, so today the pick among several defaults changes from run to run; layering makes that case easy to reach (rc says `perso`, config says `work`). An rc changing the default writes `accounts.work.default = false` beside its own.
- `target_path` keeps naming the first existing default path, the base layer, so the wizard keeps appending there. Its `ExistingConfig` reads the merged view instead of that one file, so a generated name never collides with, and is never masked by, an account in another layer, and it never adds a second default.
- The loaded layers are logged at debug level, a stale rc coming back to life being the one surprise this change can cause.

## Scope / non-goals

The code lands in pimalaya-config (`src/toml.rs`), which every Pimalaya CLI shares, so each one picks the rule up on its next pin bump; himalaya and himalaya-tui move together since they read one file. The array-concatenation rule of the merge is unchanged, for `-c` and layers alike. The other CLIs' wizards get the same `ExistingConfig` fix as they bump.
