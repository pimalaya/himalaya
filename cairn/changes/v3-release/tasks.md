---
cairn: tasks
change: v3-release
---

# Tasks

## With the maintainer

- [ ] Review every `json-schema` entry's shape; fold what changes into its command's change and spec
- [x] Publish pimalaya-cli and pimalaya-config 0.4: 0.4.0 published 2026-11-11
- [x] Publish io-http, io-jmap, io-msgraph and io-pimdir: io-http 0.6.0 and io-jmap 0.7.0 were already out, io-msgraph 0.7.1 and io-pimdir 0.10.0 published 2026-11-11
- [x] ~~Publish io-imap~~ Done: io-imap 0.8 comes from crates.io (529ed5d), out of the patch table.
- [x] ~~Publish ical-rs~~ Dropped: no longer a dependency, its patch gone with pimdir-follows-storage-draft-09 (2026-11-06).
- [x] Raise the requirements to the published versions (io-gmail 0.6 with them, for one io-http), remove `[patch.crates-io]`, confirm a clean crates.io build: clippy over the 15 feature sets, the whole suite and the build without SMTP pass
- [x] ~~Tighten deny.toml once no git source remains~~ Dropped: the owner keeps `allow-org.github = ["pimalaya"]`, git pins being common across the family (2026-11-11).
- [ ] Owner validation of the whole family, then Ortie and Himalaya released together with pimalaya-cli and pimalaya-config 0.4

## Release

- [x] MIGRATION.md: the v2 to v3 section (drafted 2026-11-02, final once the JSON review is done)
- [x] MIGRATION.md: the v2 to v3 section gains the breaking changes landed since its draft (the wizard, the table presets and colours and `-w/--max-width` removed, `-c` one path per flag, `HIMALAYA_CONFIG` removed, ASCII hosts, the plain text output)
- [ ] A new screenshot.jpeg showing the plain columns, the current one showing the v2 coloured table (postponed by the owner on 2026-11-11)
- [ ] Bump the version to 3.0.0 (the schema `$id`s follow, `v3`)
- [ ] Finalise the CHANGELOG `[Unreleased]` section into `[3.0.0]`, its breaking entries marked
- [ ] Run the manual provider reports under cairn/spec/testing against the release build, on the Google, Microsoft and Fastmail test accounts only
- [ ] Tag and publish
- [ ] Log entry; status `landed`
