---
cairn: change
id: follows-storage-draft-17
---

# Inventory

Measured on 2026-11-23 on scratch copies under ~/code/pimalaya/.pimdir-migration/himalaya/ (base: `git archive a2726f56`, untouched; src: the same with the pin alone moved to 7f16b27; proto: src plus the fixes of tasks.md step 1), CPUs 2,3,18,19, `-j 4`. Claims marked "read" come from code, unverified.

## 1. Compile breaks (pin moved, nothing else)

`cargo build --all-features --all-targets --keep-going` on src, then proto for the integration tests (they wait on the lib):

| site | error |
| --- | --- |
| src/pimdir/backend.rs:720 (lib) | E0004 `PimdirAction::SetProperty` not covered |
| src/pimdir/backend.rs:3589, :4969; src/pimdir/index.rs:786 (lib tests) | E0063 `server_object_id` missing in `PimdirPlacement` |
| tests/pimdir_search.rs:67, tests/pimdir_queue.rs:50, tests/pimdir_attachments.rs:172 | E0063, same |

No other literal or exhaustive match of a changed type (`PimdirRemoteMeta`, `PimdirBinding`, `PimdirHeldObject`, `PimdirPlanCosts`, `PimdirWriteOp`, `PimdirQueryPredicate`, `PimdirPlanEstimate`, `PimdirQueryErrorCode`): grep of src/ and tests/. Cargo.lock: io-pimdir's source and one `libc` line.

## 2. Tests

| run | passed | failed | ignored |
| --- | --- | --- | --- |
| base `cargo test --all-features` (lib 725 in 114.5 s) | 867 | 0 | 25 |
| proto, compile fixes only | 863 | 4 | 25 |
| proto, step 1 complete (lib 725 in 31.9 s) | 867 | 0 | 25 |

The four, each fixed by the line of tasks.md step 1 named after it:
- `pimdir::backend::tests::a_damaged_store_is_refused_as_corrupt`: `no such table: item_address` (backend.rs:2972).
- `pimdir::backend::tests::a_store_of_another_draft_is_refused_saying_which_is_earlier`: storage-draft-16 now reads as earlier, "move its directory aside" (backend.rs:2914).
- `pimdir::index::tests::a_hit_the_store_cannot_read_is_an_item_error`: `no such table: mail_summary` (index.rs:940).
- `a_message_the_store_cannot_read_is_an_item_error` (tests/pimdir_search.rs:218): same.

CI matrix (the 14 clippy sets of .github/workflows/tests.yml, `-D warnings`, `--locked`, then the build without SMTP, 340 tests): green on base and on proto. `cargo-deny check`: advisories red on both (RUSTSEC-2026-0592, url 2.5.8; `cargo update --dry-run -p url` offers 2.5.9 under Rust 1.89), bans, licenses and sources green. `cargo tree -e normal -i ical-rs` and `-i vcard-rs`: nothing.

## 3. SQL and stamps

Himalaya runs no SQL in production (rusqlite is a dev-dependency). Test SQL against moved tables: src/pimdir/index.rs:940, tests/pimdir_search.rs:218 (`mail_summary`), src/pimdir/backend.rs:2972 (`item_address`). Still valid: backend.rs:2256 (`queue.payload`), :2858, :2902 (`store_meta`), :2944, :3926 (`items.sort_key`), index.rs:1256, :1275 (`index_meta`). Stamps: backend.rs:2913-2914, index.rs:1275-1299 (`search-draft-11`, still earlier). Docs naming a draft: MIGRATION.md:150 (`storage-draft-14`, stale already), cairn/spec/search.md:55 (`search-draft-16`).

## 4. Behaviour, base against the migrated release build

On a Maildir probe (`envelope search --json`) and the store copies:

| input | base (dc19b8d) | migrated (7f16b27) |
| --- | --- | --- |
| `~tilde`, `/slash`, `subject:{brace}` | listed as words | `search-invalid`, `reserved-form` |
| `"~tilde"` | listed | listed |
| `label:x`, `property:a/b=c` | `search-invalid`, `unknown-field` | `search-unsupported` |
| `pimdir search label:x`, `pimdir count property:a/b=c` | refused (read) | answered, 0 |
| `pimdir search '~x'` | (read) answered | `query-refused`, `reserved-form` |
| a storage-draft-15 store | opens | `store-version`: "Pimdir store is storage-draft-15, written by an earlier sync engine, and himalaya reads storage-draft-17 alone: … sync it with the engine that wrote it until nothing is queued, then move its directory aside and sync a new store" |

Read: io-pimdir's readers open with `SQLITE_OPEN_SHARED_CACHE` at 7f16b27 (src/client/reader.rs:523); writers do not.

## 5. Performance

Fixtures: before, a copy of .bench-notmuch/runs/x1/pd/store (storage-draft-15, search-draft-16, written by io-pimdir cebe50d, whose migrations equal dc19b8d's; 518,401 items, opened by HEAD); after, a copy of .pimdir-night/sync-perf/store (storage-draft-17, search-draft-17, 518,701 items, index refreshed to current on the copy). Both: 3,312 or 3,313 mailboxes, `dasovich-j/all_documents` 11,896 messages, same first envelope, queue empty. Release builds of base and proto (default features), the `pimdir` binary of 7f16b27; hyperfine medians in ms, CPU 15 under the measure lock, 10 runs (3 for the slow rows), **while other lanes built** (load 9 to 12): indicative.

| command (`--json`; `-m dasovich-j/all_documents -s 50` on envelope rows) | before | after |
| --- | --- | --- |
| mailbox list | 16.23 | 13.78 |
| envelope list | 24.89 | 22.44 |
| envelope list -p 20 | 37.11 | 35.63 |
| envelope search from:jeff.dasovich@enron.com | 25.48 | 21.91 |
| envelope search is:unread | 22.12 | 19.60 |
| envelope search california | 49.91 | 35.30 |
| envelope search kiwi (no hit in the mailbox: every body read) | 1,473.05 | 987.38 |
| pimdir search -s 50 california | 1,363.19 | 34.92 |
| pimdir search -s 50 kiwi | 1,041.13 | 11.81 |
| pimdir search -s 50 --threads california | 2,310.52 | 39.46 |
| pimdir count california | 1,129.20 | 64.72 |
| pimdir count is:unread | 1,243.64 | 63.89 |
| pimdir mailbox list | 77.62 | 57.94 |
| pimdir status | 29,646.17 | 34,490.16 |

The `pimdir` binary on the after copy: search -n 50 california 6.40, kiwi 3.56, --threads california 7.71, count california 4.50, count is:unread 4.54; with Himalaya's scope, `count 'california kind:mail'` 5.74 and `search -n 50` 7.68 (measure lock), `count '(california) kind:mail'` 8.1 and `search -n 50` 10.9 (rough, CPU 3). Himalaya in the same run: `pimdir count california` 73.2 (40,189, the binary's count), `pimdir search -s 1 kiwi` 9.66, `mailbox list` 13.22; rough: `pimdir count kiwi` 8.3, `pimdir search -s 1|10|50 california` 17.5, 22.5, 35. `mid:none@x.org`: binary count 516, page 813; Himalaya count 533. The x1 notmuch run of 2026-11-21 (.bench-notmuch/runs/x1/queries.txt, another day, so for scale only): california count 5.6, page 15.3, threads 17.2; kiwi 2.4, 3.8, 4.8.

A frame-pointer profile of the debug build's `pimdir count california` (68 ms) puts 20.6% in `memmove` and 18.9% in `sqlite3VdbeExec`, callers cut at `lang_start`; DWARF unwinding is refused (perf_event_paranoid 2).

Read: `pimdir status` walks every summary of every mailbox to count bodies (src/pimdir/backend.rs:1487-1491); `mail_hit` reads the store once per hit (src/pimdir/index.rs:275-279); `matches` calls `match_mails` once per item (src/pimdir/backend.rs:569-596). io-pimdir's `PimdirReader::search_coverage(collections, ..)` answers a collection's bodiless count from the store (`sql::COUNT_BODILESS`).

## 6. Open lines triaged (re-checked on a2726f56 unless said)

| line | source | state |
| --- | --- | --- |
| `envelope search` from the index, `--sort relevance` there | changes/envelope-search-speaks-pimdir | open: proposal D2 |
| git pin to io-pimdir 0.20.0 | same | open: tasks.md step 7 |
| `\seen` counted unread by the store, read in the listing | log/2026-11-20-pimdir-counts-what-it-lists | open, read: marks are exact bytes (needs-io-pimdir N1) |
| an undecodable pending row errs its whole mailbox | log/2026-11-20-the-contract-audit-rulings | open, io-pimdir's (N2), `pending_actions` unchanged |
| queue keys named `collection` across the family | log/2026-11-19-pimdir-status-names-the-mailbox | out: family ruling |
| `request-origin` code, twins' handle | logs of 2026-11-19 | out: not pimdir, other repos |
| HIM-9 one batch per refile or delete | CLI analysis | closed: `refile` stages through `enqueue_all` (backend.rs:1320) |
| HIM-14 unbounded query recursion | CLI analysis | closed: the chumsky parser is gone, io-pimdir bounds 32 groups |
| HIM-17 `--seen` on pimdir exits 0 | CLI analysis | closed: `a_read_whose_seen_flag_is_refused_fails` (backend.rs:4442) |
| HIM-13 Graph half, HIM-18 schema | CLI analysis, the-schema-review-remainder | out: not pimdir (v3-release) |
| C3 attachment names | pimdir audit-2026-11-12 | closed here (log/2026-11-19-a-saved-attachment-name-is-neutralised); pimdir's box unticked |
| C5 store rules restated | same | closed here: `accounts_of`, `scoped`, `unread_counted`, `overlay_quiet`, `media_type` used (client.rs:331-349, index.rs:236, backend.rs:180, :354) |
| R4, R1, K4 | pimdir changes/a-smaller-sync/report.md | open, io-pimdir's (N3) |
| contract audit X1 to X15 | ledgers/cli-contract-audit-2026-11-20 | landed 2026-11-20 |
| GitHub issues | `gh issue list -R pimalaya/himalaya` | none open |
