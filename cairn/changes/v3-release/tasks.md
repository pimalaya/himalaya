---
cairn: tasks
change: v3-release
---

# Tasks

## With the maintainer

- [x] Review every `json-schema` entry's shape; fold what changes into its command's change and spec (2026-11-19: every schema-review finding walked, the non-breaking ones landed in the-schemas-describe-every-key; what is left for the review is in changes/the-schema-review-remainder) (2026-11-26: the 220 schemas of the 3.0.0 build reviewed, the findings in the-schema-review-remainder "Review of 2026-11-26": R1 to R5 breaking, left for the owner before the tag; R6 to R10 after it; the `IanaFlag` description fixed) (2026-11-26: R1 to R5 landed before the tag as the owner ruled, log 2026-11-26-the-schema-review-remainder; R6 to R10 stay after it in the-schema-review-remainder)
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
- [x] A new screenshot.jpeg showing the plain columns: replaced by the tested `envelope search --json` block of the README (the-pre-release-reading, 2026-11-21)
- [x] Adopt the io-imap release handing raw bytes and land HIM-18 (the-schema-review-remainder), ruled before v3 on 2026-11-19 (2026-11-19: io-imap 0.12, the-releases-of-2026-11-19)
- [x] Bump the version to 3.0.0 (the schema `$id`s follow, `v3`): 6f12217c; every `$id` reads `v3`, pinned by tests/json_schema.rs `every_schema_id_names_the_v3_contract` (2026-11-26).
- [x] Finalise the CHANGELOG `[Unreleased]` section into `[3.0.0]`, its breaking entries marked: `[3.0.0]` written by 869fe31f and 944ba0ad; checked 2026-11-26 against `git log v2.2.1..HEAD` (churn inside the major folded into the final state, as 869fe31f chose), `source-empty` added, a stray blank line removed; MIGRATION.md reread against the pimdir v1 move and the `--help` of the release build, nothing stale found. Its date stays for the lead to set.
- [ ] Run the manual provider reports under cairn/spec/testing against the release build, on the Google, Microsoft and Fastmail test accounts only
- [ ] Tag and publish
- [ ] Log entry; status `landed`
