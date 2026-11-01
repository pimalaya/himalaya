---
cairn: change
id: queue-cancel-leaves-the-schema-alone
status: landed
created: 2026-10-30
---

# `pimdir queue cancel` leaves the schema alone and the other accounts' rows too

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, release blocker 1 and should-fix 6) reproduced data loss through `himalaya pimdir queue cancel`.

- It calls io-pimdir's `PimdirStore::cancel_action`, which runs the full owner `open` (io-pimdir src/client.rs:877-885): that migrates and reconciles the schema to the caller's io-pimdir revision. A himalaya built on 3b4bed8, cancelling a row in a store written by io-pimdir HEAD, dropped the `supersessions` table, `task_summary_by_parent`, and the `priority`, `categories`, `parent` and `mid` columns of `task_summary`: unfog's data, deleted by an email client. With version skew between apps, each such open flips the schema back and forth.
- It cancels any row id in the store (src/pimdir/client.rs:122-133, src/pimdir/queue/cancel.rs:46), another account's or another app's included, while `queue show` reads those rows as `unknown`. The prompt and the output call every row a message, a flag change or a mailbox creation included.

## What

- Cancel through `PimdirStore::open_current`, which creates, migrates and reconciles nothing, then `drop_action`. A store `open_current` refuses (`NotCurrent`: absent, another schema version, another shape) is refused saying the sync engine has to open it first; a store another process owns keeps its "a sync is running" refusal.
- Before taking the owner role, read the row with `action_status` and cancel only a pending or parked row anchored on one of the account's collections; any other row is refused as not in the account's queue, and nothing is opened as owner.
- The prompt and the output name the row's action kind (`add`, `submit`, `set-flags`, ...) rather than "message", and end with a newline.

io-pimdir's own `cancel_action` should use `open_current` too; that is io-pimdir's change, not this one.
