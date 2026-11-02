---
cairn: tasks
change: v2-release
---

- [ ] Publish pimalaya-cli (including the combined token picker) and pimalaya-config to crates.io
- [ ] Publish io-imap and io-gmail to crates.io at the declared versions
- [ ] Return io-pimdir from its local path pin (the io-pimdir changes of 2026-10-30, until pushed) to a git rev, then raise its requirement to the released version (0.9 or what io-pimdir ships as) once published
- [ ] Remove the `[patch.crates-io]` block from Cargo.toml and confirm a clean crates.io build
- [ ] Tighten deny.toml once no git source remains
- [ ] Bump the version from `2.0.0-alpha.1` to the release
- [ ] Finalise the CHANGELOG `[Unreleased]` section into a dated release
- [ ] Run the manual provider reports under cairn/spec/testing against the published build
- [ ] Tag and publish

Closed on 2026-11-02: the release shipped as 2.0.0 on 2026-07-26, then 2.1.0 and 2.2.x, and the boxes above were never ticked. What is still open of them (the git dependencies, deny.toml, the provider reports) moves to cairn/changes/v3-release.
