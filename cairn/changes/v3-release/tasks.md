---
cairn: tasks
change: v3-release
---

# Tasks

## With the maintainer

- [ ] Review every `json-schema` entry's shape; fold what changes into its command's change and spec
- [ ] Publish pimalaya-cli and pimalaya-config, then io-http, io-imap, io-jmap, io-msgraph, io-pimdir and ical-rs, to crates.io
- [ ] Raise the requirements to the published versions, remove `[patch.crates-io]`, confirm a clean crates.io build
- [ ] Tighten deny.toml once no git source remains

## Release

- [x] MIGRATION.md: the v2 to v3 section (drafted 2026-11-02, final once the JSON review is done)
- [ ] Bump the version to 3.0.0 (the schema `$id`s follow, `v3`)
- [ ] Finalise the CHANGELOG `[Unreleased]` section into `[3.0.0]`, its breaking entries marked
- [ ] Run the manual provider reports under cairn/spec/testing against the release build, on the Google, Microsoft and Fastmail test accounts only
- [ ] Tag and publish
- [ ] Log entry; status `landed`
