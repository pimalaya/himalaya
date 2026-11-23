---
cairn: log
change: follows-storage-draft-17
landed: 2026-11-23
---

# Himalaya follows storage-draft-17

The pimdir backend moved from io-pimdir dc19b8d (storage-draft-15, search-draft-16) to the io-pimdir of pimdir storage-draft-17, sync-draft-15 and search-draft-17, pinned by git until its 0.20.0 release (cairn/changes/follows-storage-draft-17).

## What landed

- 66f25fa1: `cargo fmt` of two test files, red at HEAD already.
- b5c3d744: the pin on 7f16b27, `PimdirAction::SetProperty` addressed like the other item rows, `server_object_id` in the test placements, the draft stamps of the refusal test, the test SQL on `summary` and `summary_value`.
- 54828ef4: the pin on io-pimdir drop 1, cc3021bb; url 2.5.9 (RUSTSEC-2026-0592).
- 1453aa7e: the reserved forms, `label:`, `property:`, flag terms read from the store and `set-property` rows, each pinned by a test failing on dc19b8d; help texts.
- 55cf7c7c: `pimdir status` read each mailbox's refusals through `hub_id`, listing the store's 3,313 collections twice per mailbox, and walked every summary to count bodies: 32.3 s at 517k, 88.1 ms now, the JSON unchanged.
- The pin on io-pimdir drop 2, 2314084b: a pending row whose payload does not decode is one `queue list` item error named by its handle, the mailbox's other rows listed (N2); a staged copy or move into a mailbox another source holds the message in joins it on the drain (N3, K4), pinned by `a_copy_into_a_mailbox_holding_the_message_lists_one_envelope`.
- Measures (inventory.md §7): what a pimdir command costs over the `pimdir` binary is io-pimdir's `index.hits` reader compiled under `indexer` (needs-io-pimdir.md N6) and `accounts_of`'s collection listing (N7); `envelope search` over bodies is io-pimdir's tokenizer (N8).

## Capabilities moved

- [search](../spec/search.md): a reserved form is refused; labels and properties are read from the store; `pimdir search` reads search-draft-17, its flag terms from the store; an index of search-draft-16 or earlier is `index-version`; `label:` and `property:` refused by `envelope search`.
- [backends](../spec/backends.md): Himalaya reads storage-draft-17, every earlier store `store-version`; `set-property` rows in the queue views; an undecodable pending row is an item error of its own.

## Left

- N1, N6 to N8 in needs-io-pimdir.md, for the io-pimdir lane; P1 and P2 of tasks.md step 4 wait on N6.
- The cross-app check with neverest (tasks.md step 6), the swap to io-pimdir 0.20.0 (step 7).
