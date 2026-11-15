---
cairn: log
change: the-cairn-takes-the-family-shape
landed: 2026-11-15
---

# The cairn takes the family shape

Tasks X2 and X6 of the family's cli-alignment-2026-11-15 plan (findings F45, F46): housekeeping, no behaviour of Himalaya's own moving.

## What landed

- cairn/verify.sh, calendula's verifier copied unchanged; CONTRIBUTING.md names it. It reported 23 violations, fixed in the structure: the 17 provider reports of cairn/spec/testing/ gain the frontmatter their README carries (`cairn: spec`, `capability: testing`), and the delta and tasks of mbox-backend, managesieve-extraction and managesieve-support read `cairn: delta` and `cairn: tasks`, where they read `change-delta` and `change-tasks`. It now reports the cairn conformant.
- The 117 landed changes still under cairn/changes/ move to cairn/changes/archive/, beside the 43 already there; the five others stay (v3-release, imap-special-use-aliases and search-dates-can-be-relative active, one-trust-policy-per-account a draft, a-search-says-what-it-covered abandoned). The 12 mentions of a moved change in the archived proposals and tasks now read cairn/changes/archive/<id> (one of them himalaya-tui's the-tui-runs-on-himalaya, archived there the same day), and their 23 relative links into spec/ and log/ (envelope-relative-date's two among them, broken since it was archived) climb one more level; the log entries keep theirs, and the links to cairn/spec/wizard.md stay broken, that file gone with the wizard.
- pimalaya-config 0.5.2 (`version = "0.5.2"`, the lock changing that crate alone): an unknown key's error no longer lists the removed keys among the expected ones. The spec's "Unknown keys are refused" says nothing of that list, so it stands.
- Already in the family shape, checked: MIGRATION.md's "From v2 to v3", tests/exit.rs, tests/first_run.rs and tests/account.rs, the CI `features` job, and the error code table in cairn/spec/commands.md.

No capability moved.
