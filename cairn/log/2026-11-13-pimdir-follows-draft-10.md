---
cairn: log
change: pimdir-follows-draft-10
landed: 2026-11-13
---

# The pimdir backend follows storage-draft-10, sync-draft-10 and search-draft-10

Built on io-pimdir 0.11.0 from crates.io (pimdir ae8b231, the three drafts tagged on 2026-11-13), the pin of 46e8b4ea, which left the `pimdir` feature uncompiled on twenty sites.

- Enqueue: every enqueue answers `PimdirEnqueued`, its handle and the partial supports its own gate passed the action with (STORAGE §15.6), so `producer.check` before `enqueue` is gone and each action is gated once. A queue row stays the integer of its handle (`handle.id`): `queue show` and `cancel` take row ids, which io-pimdir follows unchecked, so `Reset` is matched everywhere and asked for nowhere. The three mailbox intents warn of their partial supports too, which they did not.
- Flags: `stage_flags` appends its rows through `enqueue_batch`, one transaction, where it checked every message then queued row by row, a store failing part-way leaving the first rows. A batch refused says why and not on which action, so the gate is run alone on each action then, only to name the message in the failure, which `a_refusal_part_way_stages_nothing` asserts. A batch changing nothing takes no write transaction.
- `base`: an add and a remove state the set they read, through the overlaying reader, as `base`; a set states none. The `\Seen` a read stages is an add. `a_flag_synced_while_a_change_waits_survives_all_but_a_set` lands a flag between the staging and the drain; with `base` left out it fails on the add (`["act"]` for `["\\Seen", "act"]`).
- `remove`: `if_match` and `resolves` both `None`. Himalaya reads no version before a delete and shows no conflict, so the owner parks a delete over a conflicted message; a move leaves the conflict standing. No test of Himalaya's own covers the park, which is io-pimdir's vector.
- Queue: `PimdirQueueState` gains `Unconfirmed` and `Reset`, the unconfirmed detail carried as `error`, as io-pimdir's own CLI does. `Applied` names an optional collection: a receipt outlives its collection (§15.4), so `a_performed_deletion_reads_applied_once_the_mailbox_left` replaces the test reading `unknown`, and `queue_row` lets such a row through whatever the account, no collection saying whose it is. `queue list` reads `PimdirParkedAction::unconfirmed`; `queue cancel` asks and answers that an unconfirmed row may have taken effect, and prints `unconfirmed`.
- Coverage: `missing`, `unparseable` and `encrypted` are the sums over the kinds in scope, mail alone, raw; the private `reads` and `reads_content` say whether a term reads what they hide, which gates their mention in the `Incomplete:` line. `complete` is `is_complete()` as it stands, the `behind == 0` Himalaya added being part of it now. `zone_differs` is not carried: a mail date is an instant.
- Codes: `store-reset` and `lock-replaced` relayed, in the `himalaya-error` schema.
- Tests stamping another draft move to `storage-draft-09`, `search-draft-09` and `storage-draft-11`.

Verified: `cargo fmt --check`; `cargo test --all-features --bin himalaya`, 558 passed, 1 ignored; `cargo clippy --all-features --all-targets -- -D warnings` clean, and on each backend alone with rustls-ring; `cargo check --all-features --tests`. The suites of tests/ were compiled and not run, the owner running them once every app has moved (2026-11-13). No account and no live store was reached.

Spec: backends (the staged actions, the relayed codes, `queue show`, `queue list`, `queue cancel`, the staging of flags), search (the coverage, search-draft-10, the scenario of an earlier draft), commands (the code table, the queue list shape).

Left: the suites of tests/ and an end-to-end run on a scratch store a draft-10 neverest synced. Handles as strings, a `queue retry` over io-pimdir's `retry_action`, and a view of conflicts, without which a delete parked over a conflict can only be cancelled.
