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

## N2, a pending row whose payload does not decode

- **What.** `PimdirReader::pending_actions` listing such a row instead of failing the collection (the-freeze-plan line, finding G1), unchanged at 7f16b27.
- **Why.** `pimdir queue list` reports the whole mailbox as one `action-malformed` item error, hiding its readable rows (himalaya cairn/log/2026-11-20-the-contract-audit-rulings.md, Left).
- **App test.** `an_undecodable_row_is_an_item_error_of_its_mailbox` (src/pimdir/backend.rs:2249) becomes one error for that row, the other rows listed.
- **Meanwhile.** The spec states the whole-mailbox error; kept.

## N3, the rekey and drain defects reach Himalaya's rows

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

## N4, the release

- **What.** io-pimdir 0.20.0 (or whatever version carries 7f16b27's surface) on crates.io, by its tag workflow, with the features `client`, `search`, `mail`, `indexer`, `vendored` as at 7f16b27.
- **Why.** Himalaya 3.0.0 releases on no git pin (proposal D4).
