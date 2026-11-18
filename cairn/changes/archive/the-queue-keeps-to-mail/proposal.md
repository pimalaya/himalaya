---
cairn: change
id: the-queue-keeps-to-mail
status: landed
created: 2026-11-18
---

# The queue keeps to mail

## Why

Finding TRI-1 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. `pimdir queue show` and `pimdir queue cancel` tested a row's collection against every collection of the account, with no kind filter, so a row Cardamum queued on an address book of the same account was shown and cancelled by Himalaya, while Cardamum and Calendula keep to their own kind. Rows are bare integers in one space across the three apps, so one wrong digit dropped another app's queued creation. The pimdir audit names the kind filter (C5, "the CLIs follow").

## What

- `PimdirClient::queue_row` (src/pimdir/backend.rs) tests the row's collection against the account's mail collections (`mail_collections`), so a row of another kind reads `unknown` in `queue show` and is refused by `queue cancel`, which goes through it.
