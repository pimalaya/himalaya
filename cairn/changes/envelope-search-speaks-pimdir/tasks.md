---
cairn: tasks
change: envelope-search-speaks-pimdir
---

# Tasks

- [x] Owner review, with the two io-pimdir and pimdir changes (2026-11-21: notmuch's `mid:` and orders, `unknown-field` kept, `envelope search` keeps its optional query).
- [x] `[patch.crates-io]` io-pimdir by path; io-pimdir in every build, `mail` with Maildir and mbox, `client`, `search` and `mail` on `pimdir`, never `indexer` but in `[dev-dependencies]`; `cargo tree -e normal` holds no ical-rs nor vcard-rs.
- [x] Parse with `PimdirQuery`, ariadne refusal, `search-invalid`; chumsky and the parsers removed (src/unified/email/search/parser.rs, filter/parser.rs, sort/parser.rs, eval.rs).
- [x] `--sort` onto `SearchEmailsOrder`, `pimdir search --sort` onto `PimdirSearchSort`'s five orders.
- [x] Capability options renamed to the predicate names, levels per backend, a field outside mail refused before any backend.
- [x] IMAP, JMAP, Gmail and Graph: the query lowered to the filter tree their translators read (a `Text` leaf added for free text), the translators kept. Deviation from the proposal: lowering rather than rewriting each translator over `PimdirQuery`, so their tested pushdowns stay.
- [x] Maildir and mbox through `search::mail` (`SearchEmailsQuery::matches`).
- [x] pimdir: the walk and its pushdown kept, each item matched by `search::mail` (`PimdirMail::read` or `from_summary`).
- [x] ~~pimdir answering `envelope search` from its index when the store has one, `--sort relevance` with it~~ Moved 2026-11-26 to cairn/changes/envelope-search-reads-the-index: future work, the walk answers as before.
- [x] `mid:`, instants and the orders.
- [x] Help, README block, spec search.md, MIGRATION.md, CHANGELOG.
- [x] Tests: the backend tests' queries rewritten to §8 (a one-off conversion, then reviewed), tests/envelope_search.rs, tests/capability.rs, tests/mcp.rs.
- [ ] Live runs on the Google, Microsoft and Fastmail test accounts, owner's go first.
- [x] Path patch swapped for a git pin on io-pimdir dc19b8d (2026-11-21).
- [x] Git pin replaced by a release: io-pimdir 0.20.0 (f6471b0a), then 1.0.0 from crates.io (944ba0ad); landed.
