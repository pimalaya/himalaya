---
cairn: tasks
change: the-sample-is-tested-and-shipped
---

# Tasks

- [x] config.sample.toml rewritten in Ortie's shape, every key of the old sample kept or accounted for.
- [x] Cargo.toml: pimalaya-config with `sample` as a dev-dependency.
- [x] src/config.rs: the sample test over every variant, seen failing on a broken line.
- [x] default.nix: share/doc/himalaya/config.sample.toml in the package; `nix build` checked.
- [x] install.sh: the sample copied under the prefix when the archive holds it.
- [x] README Configuration, CHANGELOG.
- [x] Fold the delta into the spec (config, packaging), log, set status landed.
