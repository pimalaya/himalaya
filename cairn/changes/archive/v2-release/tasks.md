---
cairn: tasks
change: v2-release
---

- [x] ~~Publish pimalaya-cli (including the combined token picker) and pimalaya-config to crates.io~~ Done with 2.0.0 (2026-07-26).
- [x] ~~Publish io-imap and io-gmail to crates.io at the declared versions~~ Done with 2.0.0.
- [x] ~~Return io-pimdir from its local path pin (the io-pimdir changes of 2026-10-30, until pushed) to a git rev, then raise its requirement to the released version (0.9 or what io-pimdir ships as) once published~~ Moved to v3-release: io-pimdir is on a git rev (5c18a02), its release still owed.
- [x] ~~Remove the `[patch.crates-io]` block from Cargo.toml and confirm a clean crates.io build~~ Moved to v3-release.
- [x] ~~Tighten deny.toml once no git source remains~~ Moved to v3-release.
- [x] ~~Bump the version from `2.0.0-alpha.1` to the release~~ Done with 2.0.0.
- [x] ~~Finalise the CHANGELOG `[Unreleased]` section into a dated release~~ Done with 2.0.0.
- [x] ~~Run the manual provider reports under cairn/spec/testing against the published build~~ Moved to v3-release.
- [x] ~~Tag and publish~~ Done with 2.0.0.

Closed on 2026-11-02: the release shipped as 2.0.0 on 2026-07-26, then 2.1.0 and 2.2.x, and the boxes above were never ticked. What is still open of them (the git dependencies, deny.toml, the provider reports) moves to cairn/changes/v3-release.
