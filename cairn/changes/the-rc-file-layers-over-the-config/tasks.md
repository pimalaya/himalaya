---
cairn: tasks
change: the-rc-file-layers-over-the-config
---

# Tasks

- [x] pimalaya-config: replace `first_valid_default_path` by a `valid_default_paths` returning every existing default path in layer order, deduplicated by canonical path.
- [x] pimalaya-config: `from_default_paths` merges those layers; debug-log each loaded layer.
- [x] pimalaya-config: `target_path` names the first layer, its doc comment rewritten (the shadowing rationale no longer holds).
- [x] pimalaya-config: tests for layering order, rc override, symlinked rc read once.
- [x] pimalaya-config: changelog entry under Changed.
- [x] himalaya: bump the pimalaya-config pin (5fc629b).
- [x] himalaya: error on several `default = true` accounts.
- [x] himalaya: wizard `ExistingConfig` reads the merged config.
- [x] himalaya: README configuration section and changelog.
- [ ] himalaya-tui: same pin bump.
- [x] Fold the delta into spec/config.md, log, set status landed.
