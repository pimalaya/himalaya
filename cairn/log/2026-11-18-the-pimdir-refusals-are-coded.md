---
cairn: log
change: the-pimdir-refusals-are-coded
landed: 2026-11-18
---

# The pimdir refusals are coded

Findings TRI-4, TRI-7 and TRI-8 of the family's cli-family-analysis-2026-11-15 plan, Himalaya's part.

## What landed

- src/error.rs: `store-account-ambiguous`, `store-account-unknown`, `row-not-found`, `confirmation-required`, `page-token-invalid`.
- src/pimdir/client.rs, backend.rs, queue/cancel.rs, search.rs, attachment/mod.rs: the refusals coded; the account derived from the accounts holding mail; a cancel refused before its prompt.
- Tests tests/pimdir_refusals.rs, failing on a04ff103.
- CHANGELOG (Changed, breaking), MIGRATION (development build).

## Capabilities moved

- [backends](../spec/backends.md): *pimdir reads one account*, *The pimdir subcommand reads and retracts the queue*.
- [commands](../spec/commands.md): *The JSON contract is described and versioned* (the code table).
- [search](../spec/search.md): *pimdir searches through the store's index*.
