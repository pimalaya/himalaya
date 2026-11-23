---
cairn: change
id: follows-storage-draft-17
---

# What Himalaya needs from io-pimdir

For the io-pimdir lane (io-pimdir cairn/changes/what-the-consumers-need). Read at io-pimdir 7f16b27; none blocks this migration.

## N1, a system flag spelled in another case

- **What.** A ruling: does a writer fold a system flag's case into `items.marks` (`\seen` stored as `\Seen`, STORAGE §13), or do marks stay exact bytes, `\seen` a keyword?
- **Why.** Himalaya lists `\seen` as read (`Flag::from_raw` classifies case-insensitively) while `mailbox list --counts`, `pimdir mailbox list`, `pimdir search is:unread` and `pimdir count is:unread` read `marks` and count it unread: two answers for one message.
- **Evidence.** himalaya cairn/log/2026-11-20-pimdir-counts-what-it-lists.md (Left); io-pimdir spec/migrations/storage/0001_init.sql:295-298 (marks "by their exact bytes"); since io-pimdir 62e8ab5 `is:` terms read `marks` (CHANGELOG Unreleased).
- **App test.** `a_flag_in_another_case_reads_alike_everywhere` (src/pimdir/backend.rs), written once the ruling is known.
- **Meanwhile.** Nothing: sources send `\Seen`; parked.

## N2, a pending row whose payload does not decode (landed on 2314084b)

- **What.** `PimdirReader::pending_actions` listing such a row instead of failing the collection (the-freeze-plan line, finding G1), unchanged at 7f16b27.
- **Why.** `pimdir queue list` reports the whole mailbox as one `action-malformed` item error, hiding its readable rows (himalaya cairn/log/2026-11-20-the-contract-audit-rulings.md, Left).
- **App test.** `an_undecodable_row_is_an_item_error_of_its_mailbox` (src/pimdir/backend.rs:2249) becomes one error for that row, the other rows listed.
- **Meanwhile.** The spec states the whole-mailbox error; kept.

## N3, the rekey and drain defects reach Himalaya's rows (landed on 2314084b)

- **What.** The fix of K4 (pimdir cairn/changes/a-smaller-sync/report.md), released, with its test; R4 and R1 are fixed by the io-pimdir lane in the same drop.
- **Why.** Himalaya stages `set-flags`, `move` and `copy` that neverest drains: K4 mints a second message for a staged copy or move into a mailbox another source holds it in. R4 (an owed flag change dropped after a rekey whose listing states unknown flags) does not reach Himalaya's rows through today's neverest, which rekeys IMAP alone and lists known flags (neverest's plan, inventory.md F).
- **App test.** In tests/pimdir_queue.rs, with io-pimdir's owner under the dev-dependency: a `message copy` into a mailbox another source holds the message in lists one envelope.
- **Meanwhile.** Nothing Himalaya can do: noted in report.md and CHANGELOG once released.

## N5, a `mid:` term scans

- **What.** A `mid:` term driving by the Message-ID (`summary.uid`) rather than a scan.
- **Why.** `pimdir search mid:<id>`, the script's way to find one message, takes 0.8 s at 517k.
- **Evidence.** On the 517k copy, rough runs on CPU 3: the `pimdir` binary of 7f16b27 counts `mid:none@x.org` in 516 ms and pages it in 813 ms, `california kind:mail` in 6.4 ms; Himalaya's `pimdir count mid:none@x.org` 533 ms (inventory.md §5).
- **App test.** None timing-based; tasks.md step 4 re-measures it.
- **Meanwhile.** `envelope search mid:<id>` walks one mailbox and answers in milliseconds; nothing to do here.

## N6, the hit columns read without `indexer`

Added 2026-11-23, at cc3021bb.

- **What.** The `index.hits` reader (src/client/search/hits.rs, `mod hits` and every `columnar*` call in src/client/search.rs) under `search`, not `indexer`: a query client reads the file the indexer writes, parsing no mail, vCard nor iCalendar.
- **Why.** Himalaya builds io-pimdir with `client`, `search`, `mail` (no `indexer`, which pulls ical-rs and vcard-rs into its normal graph), so every `pimdir count`, `pimdir search` and `--threads` takes the index.db path the cache exists to skip. It is P1 and most of P2 of tasks.md step 4.
- **Evidence.** 517k store copy, release builds of himalaya 55cf7c7c, hyperfine 15 runs on CPU 3 under load (rough): `pimdir count california` 80.2 ms, the same tree built `--features io-pimdir/indexer` 14.3 ms; `pimdir search -s 50 california` 39.6 against 17.7; `--threads` 47.6 against 18.6. The `pimdir` binary of cc3021bb (`--refresh-timeout 0s`): 6.6, 8.9, 10.3. A frame-pointer profile of the first puts the time in `PimdirIndex::counted` (`read_terms`, `materialize`, `run`), the `columnar` branch compiled out.
- **App test.** None timing-based; tasks.md step 4 re-measures.
- **Meanwhile.** Nothing: enabling `indexer` in Himalaya breaks proposal A (no ical-rs nor vcard-rs in the normal graph); P1 and P2 wait on this.

## N7, `accounts_of` without the coverage join

Added 2026-11-23, at cc3021bb.

- **What.** `PimdirReader::accounts_of` reading the accounts and kinds alone (`SELECT DISTINCT account, kind FROM collections`, or a statement of the same answer) rather than `list_collections`, whose coverage join groups every source.
- **Why.** Himalaya resolves its account through `accounts_of` (the C5 rule: no store rule restated), once per command: 4.3 ms at 3,313 collections (sqlite3 `.timer` on `list_collections_by_account`), most of the 7 ms Himalaya pays over the `pimdir` binary on every pimdir command at 517k.
- **App test.** None timing-based.
- **Meanwhile.** Kept on `accounts_of`.

## N8, a text term tokenizes without allocating (optional)

Added 2026-11-23, at cc3021bb.

- **What.** `search::fold::holds` matching a word without collecting the text's tokens into a `Vec<String>` (`fold::tokens`).
- **Why.** `envelope search kiwi` on a pimdir account walks every body of the mailbox (1 s at 11,896 messages, D2): a frame-pointer profile puts 46% in `match_mails`, 37% of it in `fold::tokens` (19% self in its token iterator), the rest of `PimdirClient::matches` (25%) reading the bodies; the per-call cost of one `match_mails` per message does not show, so batching the calls (P3) gains nothing.
- **App test.** None timing-based.
- **Meanwhile.** Recorded, not gated (proposal D2).

## N4, the release

- **What.** io-pimdir 0.20.0 (or whatever version carries 7f16b27's surface) on crates.io, by its tag workflow, with the features `client`, `search`, `mail`, `indexer`, `vendored` as at 7f16b27.
- **Why.** Himalaya 3.0.0 releases on no git pin (proposal D4).
