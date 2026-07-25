---
cairn: tasks
change: v2-release
---

- [ ] Publish pimalaya-cli (including the combined token picker) and pimalaya-config to crates.io
- [ ] Publish io-imap and io-gmail to crates.io at the declared versions
- [ ] Remove the `[patch.crates-io]` block from Cargo.toml and confirm a clean crates.io build
- [ ] Bump the version from `2.0.0-alpha.1` to the release
- [ ] Finalise the CHANGELOG `[Unreleased]` section into a dated release
- [ ] Run the manual provider reports under cairn/spec/testing against the published build
- [ ] Tag and publish
