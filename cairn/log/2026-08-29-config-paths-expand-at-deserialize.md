---
cairn: log
change: config-paths-expand-at-deserialize
landed: 2026-08-29
---

# Every path key expands as the configuration is read

`maildir.root = "~/Mail"`, the spelling `config.sample.toml` documents, opened a directory literally named `~` under the working directory. `m2dir.root` and the `tls.cert` of every backend read the same way. `pimdir.root` and `downloads-dir` worked only because the one reader of each called `shellexpand` first, which is the shape the bug is made of: expansion at a call site holds where somebody remembered it and nowhere else.

The three roots now carry `#[serde(deserialize_with = "pimalaya_config::toml::shell_expanded_path")]`, and `downloads-dir` and `tls.cert` a private `opt_shell_expanded_path` beside them, pimalaya-config shipping no optional variant yet. `PimdirClient::new` and `Account::downloads_dir` dropped their own expansion.

Two round-trip tests cover it: a `~` root under each of the three local backends resolves under `$HOME`, and an optional path expands when written and stays absent when not.

Landed alongside two items of the family CLI alignment: comfy-table is now reached through `pimalaya_cli::table` and the direct dependency is gone, and the fully qualified paths written inline where a `use` belongs were replaced. Neither moves a requirement.

A `pimdir`-only build was also fixed: the client-side search evaluation the backend calls was gated on the `maildir` and `m2dir` features alone, so `--no-default-features --features pimdir` did not compile.

Verified against the twelve-account `~/.himalayarc`, `account list` and `account check` read-only.

Spec updated: `config` (ADDED "Path keys expand as the configuration is read"), `backends` (REMOVED "pimdir store path is shell-expanded", subsumed).
