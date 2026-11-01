---
cairn: log
change: queue-cancel-leaves-the-schema-alone
landed: 2026-10-30
---

# `pimdir queue cancel` leaves the schema alone

From the pimdir stable-release review of 2026-10-30, which reproduced the loss: a 3b4bed8 build cancelling a row in a store written by io-pimdir HEAD dropped unfog's `supersessions` table, `task_summary_by_parent` and four `task_summary` columns, because io-pimdir's `cancel_action` runs the full owner `open`, which reconciles the schema to the caller's revision. The cancel now reads the row with `action_status` first, takes only a pending or parked row anchored on one of the account's collections, then opens the store with `open_current`, which creates, migrates and reconciles nothing, and calls `drop_action`. A store of another schema is refused saying the sync engine has to open it first. The prompt and the output name the row's action kind (`kind` under `--json`) rather than calling every row a message, and the plain output ends with a newline.

io-pimdir's own `cancel_action` still runs the full open; that is io-pimdir's change.

Tests: `a_cancel_leaves_a_store_of_another_revision_alone` adds a table the canonical schema lacks (through a `rusqlite` dev-dependency, already in the graph through io-pimdir) and checks it survives a refused cancel; `a_cancel_refuses_another_accounts_row` cancels from one account a row queued by another. Both failed on the old code with `unwrap_err()` on `Ok(true)`: the cancel went through.

Capabilities moved: **backends** (The pimdir subcommand reads and retracts the queue).
