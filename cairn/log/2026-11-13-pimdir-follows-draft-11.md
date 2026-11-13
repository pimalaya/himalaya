---
cairn: log
change: pimdir-follows-draft-11
landed: 2026-11-13
---

# The pimdir backend follows storage-draft-11, sync-draft-11 and search-draft-11

Built on io-pimdir cd33f0e (pimdir d8800fb, the fourteen lines draft-10 parked, decided on 2026-11-13), its version still 0.11.0 and not on crates.io: the dependency line stays `0.11` and `[patch.crates-io]` pins the commit. Cargo.lock moves the one source. The `pimdir` feature stopped compiling on one site, a test.

- Draft names: a draft-10 store or index is the earlier one. The tests stamping another draft move to `storage-draft-10`, `search-draft-10` and `storage-draft-12`, the last because `storage-draft-11` is now the draft read.
- Coverage: `bodiless` joins `missing` and `unparseable`. It is carried whatever the query and named in the `Incomplete:` line only while a term reads a message's body (`reads`) or the query holds `has:attachment` (`reads_mark`, from `PimdirSearchReads::counts_bodiless`); it was named under every query. `complete` is `is_complete()` as it now reads, so a mailbox listed and not fetched answers `is:flagged`, a `date:` or `from:alice@x.org` whole. `a_bodiless_message_is_incomplete_only_while_a_term_reads_it` runs seven queries over the index; with the count said under every query it fails on `is:flagged`, with the mark ignored on `has:attachment`.
- Tests that stated the rule that moved: `coverage_renders_as_camel_case_json` expected the bodiless count said while no term reads a body, and now expects it left out; `a_hit_prints_on_one_line` builds its coverage with `reads` set, its query being a free word. `an_unread_body_is_incomplete_only_while_a_term_reads_bodies` built `PimdirSearchReads` by a literal, which gained `mark`: it takes `PimdirSearchReads::text()`.
- A move over a conflict: `move_messages`, which `message move` and the trash half of `message delete` call, queues a `move`. The owner parks it over a message a sync left in conflict (SYNC §7) where draft-10 applied it under the conflict, and the reader's overlay folds nothing for it (STORAGE §15.4). No production code moved: the row was already listed by `pimdir queue list` in the mailbox it was queued in and read by `pimdir queue show`, as a delete over a conflict is since draft-10. `a_move_over_a_conflicted_message_stays_listed_and_parks` holds it; on io-pimdir 0.11.0, the pin taken out for one run, it fails on the overlay (the draft gone from its mailbox) and, those two lines aside, on the drain (applied 1, parked 0). The help of `pimdir queue list` names the case.
- A group standing as a conjunct is read through, `subject:a@b` is a text term, `thread:` reads a body: io-pimdir's, Himalaya holding no check of its own. `scope` still hands the index the conjuncts at the top.
- A mail date in another shape is stored as none: Himalaya writes no summary. `a_row_out_of_date_order_searches_the_whole_mailbox`, seeding a date with an offset, still reaches the whole-mailbox read, the row now holding no date under a key that is not empty.

Verified: `cargo fmt --check`; `cargo test --all-features --bin himalaya`, 560 passed, 1 ignored (558 before, the two tests above added); `cargo clippy --all-features --all-targets -- -D warnings` clean, and on `--no-default-features --features pimdir,rustls-ring`; `cargo check --all-features --tests`. The suites of tests/ were compiled and not run. No account and no live store was reached.

Spec: backends (the overlay, the staged `Move`, the scenario of a move over a conflict), search (search-draft-11, the coverage and its scenario, the scenario of an earlier draft). commands and README name neither a draft nor the rule, and did not move.

Left: the suites of tests/ and an end-to-end run on a scratch store a draft-11 neverest synced; the pin, dropped for the io-pimdir release carrying draft-11. A view of conflicts and a `queue retry`, without which a move or a delete parked over a conflict can only be cancelled.
