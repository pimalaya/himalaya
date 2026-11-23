---
cairn: log
change: follows-storage-draft-17
landed: 2026-11-23
---

# Himalaya on io-pimdir drop 3

Follows 2026-11-23-follows-storage-draft-17.md: the pin moves to io-pimdir 17d4ea80 (drop 3, requirement 0.20.0), the cross-app check runs, and a pending row names its message again.

## What landed

- The pin on 17d4ea80: io-pimdir maps the hit columns without `indexer` (N6) and lists accounts without the coverage join (N7). `pimdir count california` 6.68 ms against the drop-3 `pimdir` binary's 5.43, `search -s 50` 10.23 against 7.55, `--threads` 11.37 against 9.03 at 517k (inventory.md §7): P1 and P2 of tasks.md step 4 met with no change in Himalaya.
- A pending `move` or `remove` row printed `subject: null`, the reader's queue overlay hiding its message from the mailbox the row waits on; the subject falls back to the committed item, pinned by `a_pending_move_names_its_subject`.
- The cross-app check (tasks.md step 6) on the lead's shared store, neverest and Himalaya on 17d4ea80: Himalaya read what neverest synced, staged a `set-flags` and a `move`, neverest applied both, and Himalaya read them `applied` with the messages where they went (journal.md, 18:58).

## Capabilities moved

- [backends](../spec/backends.md): the scenario of a pending move naming its message.

## Left

- N1 (a flag spelled in another case), N8 (optional), the swap to the released io-pimdir 0.20.0 (tasks.md step 7), the quiet-window re-run of inventory.md §5.
