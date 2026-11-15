---
cairn: log
change: the-nix-package-lives-here
landed: 2026-11-15
---

# The Nix package lives here

Task H13 of the family's cli-alignment-2026-11-15 plan (finding F46).

## What landed

- package.nix in Cardamum's shape, and default.nix calling it, where it overrode nixpkgs' himalaya derivation. Evaluated against the locked nixpkgs through default.nix with a stub `mkDefault`; not built.
- CLAUDE.md holding `@AGENTS.md`; AGENTS.md and CONTRIBUTING.md name the src/lib.rs header as the architecture document.
- CHANGELOG.

## Capabilities moved

- [packaging](../spec/packaging.md): *The Nix package lives in the repository*.
