---
cairn: change
id: the-nix-package-lives-here
status: landed
created: 2026-11-15
---

# The Nix package lives here

## Why

Task H13 of the family's cli-alignment-2026-11-15 plan (finding F46, approved optional tidying). Cardamum and ortie keep their derivation as package.nix and their CLAUDE.md as `@AGENTS.md`; Himalaya's default.nix overrode nixpkgs' himalaya derivation ("needed until the v2.1.0 derivation lands on nixpkgs's master"), and had no CLAUDE.md. AGENTS.md and CONTRIBUTING.md named the src/main.rs header as the architecture document, which lives in the src/lib.rs header since the library target.

## What

- package.nix, Cardamum's shape: the system SQLite linked only with the defaults off, `pimdir` named and `vendored` left out, its rpath set, the mingw SQLite built with pthreads; completions, manual pages, JSON Schemas and the sample installed, in Himalaya's order of the standard commands. default.nix calls it as Cardamum's does.
- CLAUDE.md holding `@AGENTS.md`.
- AGENTS.md and CONTRIBUTING.md name the src/lib.rs header.
