---
cairn: delta
change: the-rc-file-layers-over-the-config
---

# Delta

## ADDED Requirements

### Requirement: One default account
Resolving the default account SHALL fail, naming them, when several accounts are marked `default = true` once the configuration is loaded, layers and `-c` files merged. An account named with `-a` SHALL still resolve.

The wizard SHALL check the name and the default of the account it generates against the merged configuration, not only against the file it appends to.

## MODIFIED Requirements

### Requirement: Config loading and merge
Without `-c/--config`, the config SHALL load every existing canonical path as a layer, in this order: `$XDG_CONFIG_HOME/himalaya/config.toml`, `$HOME/.config/himalaya/config.toml`, `$HOME/.himalayarc`. Each layer SHALL be deep-merged on top of the ones before, so `$HOME/.himalayarc` overrides. Paths naming the same file SHALL be read once.

`-c/--config` SHALL replace the canonical paths entirely. Multiple `-c` paths MAY be passed, delimited like `$PATH` (`;` on Windows, `:` elsewhere) or repeated as flags: the first is the base and the rest are deep-merged on top.

## REMOVED Requirements

None.
