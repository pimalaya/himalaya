---
cairn: change
change: pimdir-root-shell-expand
---

## ADDED Requirements

### Requirement: pimdir store path is shell-expanded
The pimdir backend SHALL expand `~` and environment variables on `pimdir.root`
before opening the store and its blob reader, so a store path written with `~`
(e.g. a Neverest store at `~/.local/state/neverest/<account>`) resolves to the
home-relative directory. Opening the raw path would create an empty store at a
literal `./~/…` and silently return an empty mailbox list.
