---
cairn: log
change: the-rc-file-layers-over-the-config
landed: 2026-11-03
---

# The rc file layers over the config

With both `~/.config/himalaya/config.toml` and `~/.himalayarc` on disk, only the first was read: pimalaya-config's `first_valid_default_path` stopped at the first default path that existed. The default paths are now layers, each existing one deep-merged on top of the ones before, so `~/.himalayarc` overrides, as `~/.gitconfig` does over `$XDG_CONFIG_HOME/git/config`. Reading the rc file alone when present was rejected: it moves the shadowing to the other file.

pimalaya-config (src/toml.rs): `first_valid_default_path` is replaced by `default_paths` (the three candidates, lowest precedence first) and `valid_default_paths` (the existing ones, deduplicated by canonical path, since an unset `$XDG_CONFIG_HOME` makes the first two equal and a symlinked rc is common, and merging a file onto itself doubles every array). `from_default_paths` merges them and debug-logs each layer; `target_path` names the first layer. Read and parse errors now name the file. Breaking for the trait, changelog under Changed.

himalaya: `Config` overrides `take_account` to refuse several `default = true` accounts when the default is resolved, `accounts` being a `HashMap` whose iteration order changes from run to run; `-a` still resolves. The wizard's `ExistingConfig` reads the merged configuration (the `-c` paths or the default layers), so the generated name and default account account for every layer, not only the file appended to.

Verified: 28 pimalaya-config tests and the himalaya suite pass with all features, clippy clean on both. The layering test fails against the old loader (account `c` from the rc file missing), the wizard test against the old single-file read (`perso` unseen, no default), the default test against the old `take_account` (no error). End to end under a scratch `$HOME`: both files' accounts listed, an rc moving the default wins, two defaults refused with both names, a symlinked rc loaded once.

Spec: config (Config loading and merge modified; One default account added).

himalaya pins pimalaya-config at 5fc629b, the commit carrying the layers. Left: himalaya-tui, reading the same file, follows with the same bump. The other CLIs' wizards have the same single-file `ExistingConfig` shape and get the layers on their next bump.
