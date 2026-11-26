---
cairn: change
id: follows-storage-draft-17
part: report
---

# Report

Himalaya runs on pimdir storage-draft-17, sync-draft-15 and search-draft-17 through io-pimdir 17d4ea80 (drop 3, requirement 0.20.0, git patch); run on 2026-11-23 by the himalaya lane. No silent loss found.

## What landed

- 66f25fa1 `style`: fmt was red at HEAD already.
- b5c3d744 `build!`: the move to 7f16b27, seven compile sites, the draft stamps and the test SQL.
- 54828ef4 `build`: drop 1 (cc3021bb), url 2.5.9 (RUSTSEC-2026-0592).
- 1453aa7e `feat(search)!`: reserved forms refused, `label:` and `property:`, flag terms read from the store, `set-property` rows in the queue views.
- 55cf7c7c `perf(pimdir)`: `pimdir status` from 32 s to 88 ms at 517k. The cost was not the body walk the plan suspected: `refused_rows` resolved every mailbox through a listing of all 3,313 collections, twice per mailbox.
- 13bb4627 `fix(pimdir)!`: drop 2 (2314084b). An undecodable queue row is its own item error (N2). A copy into a mailbox another source holds joins the held message (N3, K4); its test fails on drop 1.
- 288ef2f7 `docs(cairn)`: spec fold, CHANGELOG (BREAKING lines), MIGRATION.md (a store a development build wrote is refused: drain it, move it aside, sync anew), log entry, measures.
- f6471b0a `build`: drop 3 (17d4ea80), requirement 0.20.0.
- a343edae `fix(pimdir)`: a pending move or remove row names its message again (found in the cross-app check); P1, P2 and step 6 closed.

## Measures (517k Enron store, indicative, under load)

- Before and after the migration: `pimdir search -s 50 california` 1,810 to 10.2 ms, `--threads` 2,710 to 11.4, `pimdir count california` 1,234 to 6.7, `pimdir status` 39,646 to 80.
- Each pimdir query is within 2.7 ms of the drop-3 `pimdir` binary (inventory.md §7). What closed the gap was in io-pimdir, both in drop 3: the hit columns had been compiled only with `indexer` (N6), and `accounts_of` ran a coverage join on every command (N7).
- `envelope search mid:` read 218.7 then 333.6 ms in the loaded run, the only row slower by more than 5%; it needs a quiet-window run (not done, load 12) before it reads as a regression.

## Cross-app check

On the shared store, with neverest and Himalaya both on 17d4ea80, Himalaya read what neverest synced and staged a `set-flags` and a `move`; neverest applied both; Himalaya then read both rows `applied`, the flag on its message, and the moved message in the archive under the same public id (journal.md, 18:58).

## Waiting on others

- N1, a system flag spelled in another case: waits on the owner's STORAGE §13 ruling.
- N8 (optional): `fold::tokens` allocating, most of what `envelope search` over bodies costs.
- N4: io-pimdir 0.20.0 released by its tag workflow; step 7 then drops the git patch. Himalaya 3.0.0 releases only on that version (D4), with neverest, cardamum, calendula and carillon on it too.

## Decisions taken (defaults unless said)

- D1 to D5 as proposed.
- N2 shape: the bad row is an item error under its handle, not a listed row carrying the decode error as the `pimdir` binary prints it, so `errors` and exit 3 keep their contract (journal 18:25).
- The windows-sys lines the MSRV resolver flipped in Cargo.lock were put back by hand (journal 16:49).
