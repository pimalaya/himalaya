---
cairn: tasks
change: config-paths-expand-at-deserialize
---

# Tasks

- [x] Put `shell_expanded_path` on `maildir.root`, `m2dir.root` and `pimdir.root`.
- [x] Hand-roll `opt_shell_expanded_path` and put it on `downloads-dir` and `tls.cert`.
- [x] Drop the expansion in `PimdirClient::new` and `Account::downloads_dir`.
- [x] Round-trip tests: a `~` root resolves under `$HOME`, an absent optional path stays absent.
- [x] Fold the pimdir-only requirement into the config capability.
- [x] Changelog entry.
