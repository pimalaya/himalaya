---
cairn: log
change: docs-aligned-for-the-major
landed: 2026-11-25
---

# Aligned the docs on Ortie's for 3.0.0

## What landed

- README.md: Ortie's shape (header, its existing tested JSON block, unchanged, Features, install from pkg.pimalaya.org, Homebrew, installer, Cargo, Nix and sources, configuration template, provider recipes, numbered first run, Alternatives); tests/docs.rs checks the Features budget and the section order.
- CHANGELOG.md: the Unreleased section rewritten to changelog-001 and changelog-002, a lead paragraph for 3.0.0, one item per bullet, short bodies, churn folded into the final state.
- SECURITY.md: Ortie's text, 3.x supported.
- CONTRIBUTING.md: Ortie's reading order; sections restating the org guide removed, overlong paragraphs split.
- MIGRATION.md: tightened on ortie's (457 to 178 lines), every action kept, stale facts corrected against `--help`.
- Capabilities moved: none (documentation only).
