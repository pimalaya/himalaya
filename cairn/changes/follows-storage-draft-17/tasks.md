---
cairn: tasks
change: follows-storage-draft-17
---

# Tasks

Every cargo run: `CARGO_TARGET_DIR=target CARGO_PROFILE_DEV_DEBUG=0 taskset -c 2,3,18,19 nix develop --command cargo … -j 4`. "The gate" below means `cargo test --all-features` (867 passed, 0 failed, 25 ignored before this change, plus the tests a step adds), the 14 clippy sets of .github/workflows/tests.yml with `-D warnings`, `cargo test --no-default-features --features rustls-ring,maildir` (340) and `cargo fmt --check`. Each step that ends in a commit commits unsigned per frame.md §4; decisions go to journal.md in this folder, timed.

## 0. Start

- [x] Read frame.md, proposal.md, inventory.md; create journal.md (and later report.md) with the frontmatter `cairn: change`, `id: follows-storage-draft-17`, which cairn/verify.sh requires of every file here. Gate: `git status` clean at a2726f56 or a later owner head.

## 1. Build on 7f16b27 (A to D, one commit)

- [x] Cargo.toml:83 rev `7f16b27da3b11a42b9d81a522a3e6fa5eeeed994`; `cargo update -p io-pimdir`. Gate: the Cargo.lock diff is the io-pimdir source and its `libc` line; `cargo tree -e normal -i ical-rs` and `-i vcard-rs` print nothing.
- [x] src/pimdir/backend.rs:736-739 (`addressed`): `| PimdirAction::SetProperty { seq, .. }` joins the arm returning `stored(*seq)`.
- [x] `server_object_id: None` at src/pimdir/backend.rs:3589 and :4969, src/pimdir/index.rs:786, tests/pimdir_search.rs:67, tests/pimdir_queue.rs:50, tests/pimdir_attachments.rs:172.
- [x] src/pimdir/backend.rs:2913-2914: stamp `storage-draft-11` and `storage-draft-15` (move its directory aside), `storage-draft-99` (upgrade himalaya).
- [x] src/pimdir/index.rs:940 `UPDATE summary SET size = 'x' WHERE item = …`; tests/pimdir_search.rs:218 `UPDATE summary SET size = 'x' WHERE component = 'mail' AND title = 'Report 2'`; src/pimdir/backend.rs:2972 `DROP TABLE summary_value`.
- [x] Gate: the four tests of inventory.md §2 pass, then the whole gate. Commit `build!: follow pimdir storage-draft-17 and search-draft-17`.
- [x] D5: `cargo update -p url` (2.5.9, form_urlencoded 1.2.3). Gate: `cargo-deny check` green. Commit `build: url 2.5.9 (RUSTSEC-2026-0592)`.

## 2. Inherited behaviour, pinned (E; each test fails on dc19b8d, inventory.md §4)

- [x] tests/envelope_search.rs `a_reserved_form_is_refused_with_its_code`: on Maildir, `~tilde`, `/slash`, `subject:{brace}` exit 1 `search-invalid` naming `reserved-form`; `'"~tilde"'` lists the message.
- [x] src/pimdir/index.rs tests: `a_reserved_form_is_refused_as_query_refused` (`pimdir search /x`); `a_label_is_read_from_the_store` (owner `add_property` of `pimdir/label` `todo` on one message: `label:todo` lists it; `envelope search label:todo` is `search-unsupported`); `a_flag_term_reads_the_store_while_the_index_lags` (owner sets `\Flagged` after the refresh: `pimdir count is:flagged` counts it, `coverage.behind` > 0).
- [x] src/pimdir/backend.rs `a_set_property_row_names_its_message`: a producer-queued `set-property` on imap/INBOX lists with `kind`, `itemId`, `subject`; `queue show` pending; `queue cancel --yes` drops it.
- [x] Help: src/unified/envelope/search.rs QUERY help names the reserved forms; src/pimdir/search.rs says flag terms read the store as the sync engine last wrote it and names `label:`, `property:`; tests/mcp.rs follows the `envelope_search` description if it changes.
- [x] Gate. Commit `feat(search)!: follow search-draft-17 (reserved forms, label:, property:)`.

## 3. Bugs (G)

- [x] `pimdir status` takes 30 s at 517k, walking every summary to count bodies (src/pimdir/backend.rs:1487-1491): on a mailbox `overlay_quiet` answers quiet, `hydrated` = `count_items` minus the `bodiless` of `PimdirReader::search_coverage(&[id], None, None)` (`sql::COUNT_BODILESS`), the walk kept for a busy queue, as `count_unread` does. Test `a_status_counts_bodies_as_the_walk_does` (quiet and busy fixtures, a bodiless item in each). Gate; threshold under 500 ms at 517k; commit `perf(pimdir): status counts bodies without reading every summary`.
- [ ] N1 parked; N2, N3 landed on drop 2 (journal 18:25). N1, N2, N3 (needs-io-pimdir.md): once io-pimdir releases each, add its app test (named there), make it pass, gate, commit.
- [x] Re-check the closed lines of inventory.md §6 on the head of this step; reopen any that fails.

## 4. Performance (H)

- [x] (closed by N6, N7 on drop 3, journal 18:56) P1 `pimdir count california` 73 ms where the `pimdir` binary counts `(california) kind:mail` in 8.1 ms (inventory.md §5), the gap growing with the hits (`kiwi` 8.3 ms): profile a release build with `CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` under `perf record -e cpu-clock --user-callchains -g` (DWARF unwinding is refused at perf_event_paranoid 2); first suspect, the store opened twice in shared-cache mode (`PimdirClient`'s reader and the index's attach). Fix the cause; threshold within 5 ms of the binary.
- [x] (closed by N6, N7 on drop 3, journal 18:56) P2 `pimdir search -s 50 california` 35 ms against the binary's 10.9 ms, about 0.4 ms per hit: `mail_hit` (src/pimdir/index.rs:268-297) reads `get_item` per hit; build the envelope from one read per page, or from what the hit carries; same threshold.
- [x] P3 `envelope search` matching: src/pimdir/backend.rs:569-596 calls `match_mails(&[mail])` per item (`kiwi`, 11,896 bodies, 987 ms); profile as P1 and batch per walked page when the call is over 10% of the samples.
- [ ] (measured under load, inventory.md §7; quiet re-run at the lane's end) Re-run inventory.md §5 in a quiet window per frame.md §5 (three copies, A/A within 1%, interleaved; notmuch on .bench-notmuch/runs/x1 beside). Thresholds: no row slower than before beyond 5%; `pimdir search|count|--threads` within 5 ms of the `pimdir` binary on the same store; `pimdir status` under 500 ms at 517k; `envelope list -s 50` under 25 ms; `envelope search kiwi` and `mid:` recorded, not gated (D2, N5). Fixtures: the two copies of inventory.md §5, rebuilt by the same `cp -a` if gone (the draft-15 one from .bench-notmuch/runs/x1/pd/store, else imported from ~/code/pimalaya/.pimdir-night/sync-perf/corpus with `pimbench sync`, io-pimdir's bench/ crate whose `path = ".."` dependency points at an archive of dc19b8d). Table into inventory.md, commit `docs(cairn): measures after storage-draft-17`.

## 5. Docs (I)

- [x] Fold delta.md into cairn/spec/search.md (:12, :55, :119-130, the two new requirements) and backends.md (:223-241 store-version, :246-290 the queue views).
- [x] MIGRATION.md:150 reads storage-draft-17; "From a development build of v3" gains: the store is refused, drain with the old neverest until `pimdir queue list` is empty, move the directory aside, sync anew with a neverest on the same io-pimdir; reserved forms; `label:`, `property:`.
- [x] CHANGELOG.md Unreleased: BREAKING store refused (storage-draft-17, search-draft-17); BREAKING reserved forms; Added `label:`, `property:` in `pimdir search`; Changed faster `pimdir search|count|--threads`; Fixed per step 3. README: no draft named (checked), unchanged.
- [x] cairn/log/2026-MM-DD-follows-storage-draft-17.md; `sh cairn/verify.sh`. Commit `docs(cairn): fold follows-storage-draft-17`.

## 6. Cross-app check (J; servers lock, never a real account)

- [x] (done on the lead's shared store, journal 18:58) Under `flock ~/code/pimalaya/.lanes/locks/servers`: neverest at its migrated head (read-only checkout, its own build, `tests/pasta.sh servers`) syncs a Stalwart test account into a scratch store; Himalaya reads it (`mailbox list`, `envelope list`, `pimdir search`, `pimdir status`); stages `flag add`, `message move`, `message add`, `message send`; neverest syncs; `pimdir queue show` of each handle reads `applied`, the envelope listing shows the result. Evidence in journal.md.

## 7. Release (K, the owner's)

- [ ] io-pimdir 0.20.0 released by its tag workflow (N4); neverest, cardamum, calendula and carillon on it.
- [ ] Cargo.toml: io-pimdir `0.20.0` in both tables, `[patch.crates-io]` removed; gate; commit `build: io-pimdir 0.20.0`. Closes envelope-search-speaks-pimdir's last task.
- [ ] v3-release: bump, CHANGELOG `[3.0.0]`, provider reports, tag, by the owner.

## 8. Report (L)

- [ ] report.md here: what landed (hashes), the measures, what waits on io-pimdir, the decisions taken; status `landed` once step 7's pin swap is in.
