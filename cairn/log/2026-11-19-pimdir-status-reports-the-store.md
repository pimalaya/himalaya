---
cairn: log
change: pimdir-status-reports-the-store
landed: 2026-11-19
---

# pimdir status reports the store

The owner's ruling of 2026-11-19 on X18 of the family's cli-alignment-2026-11-15 plan: `pimdir queue list` keeps its `refused` key and row shape, and Himalaya gains the twins' `pimdir status` at 3.0.

## What landed

- src/pimdir/status.rs: `PimdirStatusCommand` and `PimdirStatusOutput` (`{account, accounts, mailboxes, parked, refused}`), the text through `Listing` with a block for the parked rows (`parked` or `unconfirmed`) and one for the refusals.
- src/pimdir/backend.rs: `PimdirClient::status`, the mailboxes by id with `count_items`, the bodies counted off the summaries, the queued creations and sends, the parked rows with their handle and the refusals.
- src/pimdir/cli.rs, src/pimdir/mod.rs, src/json_schema.rs (`himalaya-pimdir-status`).
- Tests: `a_status_prints_mailboxes_parked_rows_and_refusals` (new command, nothing to fail against before it), and the parked-move store test asserting the report's parked row and counts.
- CHANGELOG (Added), MIGRATION (New).

## Capabilities moved

- backends: *pimdir status reports the store* (ADDED).

## Left

- The `request-origin` code of X16 is pimalaya-cli's; Himalaya's `api_url` refusal (src/shared/raw_json.rs) is an uncoded `bail!` today, nothing to rename: wrap it in that code once pimalaya-cli releases it.
