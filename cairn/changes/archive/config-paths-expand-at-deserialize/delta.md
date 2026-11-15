---
cairn: delta
change: config-paths-expand-at-deserialize
---

# Delta

Which spellings a path key accepts is user-visible, so the rule moves to the config capability. The pimdir requirement it replaces is removed from the backends capability, the behaviour it described now holding for every path key rather than for one backend.

## ADDED Requirements

### Requirement: Path keys expand as the configuration is read
Every path-valued key SHALL expand `~` and environment variables while the configuration is deserialized, not where it is read. This covers `maildir.root`, `m2dir.root`, `pimdir.root`, the global and per-account `downloads-dir`, and the `tls.cert` of every backend.

An expansion that fails, an undefined variable being the case, SHALL leave the raw value untouched rather than fail the load.

A reader SHALL therefore receive an already-expanded path and SHALL NOT expand again: expansion at a call site holds only where somebody remembered it, which is how `maildir.root = "~/Mail"` came to open a literal `./~/Mail`.

## MODIFIED Requirements

None.

## REMOVED Requirements

### Requirement: pimdir store path is shell-expanded
Subsumed by "Path keys expand as the configuration is read", which holds for every path key rather than for `pimdir.root` alone.
