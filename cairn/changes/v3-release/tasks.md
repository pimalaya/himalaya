---
cairn: tasks
change: v3-release
---

# Tasks

## With the maintainer

- [ ] Review every `json-schema` entry's shape; fold what changes into its command's change and spec
- [ ] Publish pimalaya-cli and pimalaya-config 0.4, prepared in ../cli and ../config and patched here by path
- [ ] Publish io-http, io-jmap, io-msgraph and io-pimdir, pinned here by git rev
- [x] ~~Publish io-imap~~ Done: io-imap 0.8 comes from crates.io (529ed5d), out of the patch table.
- [x] ~~Publish ical-rs~~ Dropped: no longer a dependency, its patch gone with pimdir-follows-storage-draft-09 (2026-11-06).
- [ ] Raise the requirements to the published versions, remove `[patch.crates-io]`, confirm a clean crates.io build
- [ ] Tighten deny.toml once no git source remains
- [ ] Owner validation of the whole family, then Ortie and Himalaya released together with pimalaya-cli and pimalaya-config 0.4

## Release

- [x] MIGRATION.md: the v2 to v3 section (drafted 2026-11-02, final once the JSON review is done)
- [ ] MIGRATION.md: the v2 to v3 section gains the breaking changes landed since its draft (the wizard, the table presets and colours and `-w/--max-width` removed, `-c` one path per flag, `HIMALAYA_CONFIG` removed, ASCII hosts, the plain text output)
- [ ] Bump the version to 3.0.0 (the schema `$id`s follow, `v3`)
- [ ] Finalise the CHANGELOG `[Unreleased]` section into `[3.0.0]`, its breaking entries marked
- [ ] Run the manual provider reports under cairn/spec/testing against the release build, on the Google, Microsoft and Fastmail test accounts only
- [ ] Tag and publish
- [ ] Log entry; status `landed`
