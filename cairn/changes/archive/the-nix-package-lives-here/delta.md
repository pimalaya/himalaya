---
cairn: delta
change: the-nix-package-lives-here
---

# Delta

## ADDED Requirements

### Requirement: The Nix package lives in the repository
default.nix SHALL build Himalaya from package.nix, the derivation the repository keeps as a replacement for the nixpkgs one, as cardamum and ortie do: it SHALL link the system SQLite only with the defaults off, `pimdir` asked for by name and `vendored` left out, and install the shell completions, the manual pages, the JSON Schemas and the sample.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
