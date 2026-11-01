---
cairn: log
change: pimdir-stable-release-fixes
landed: 2026-10-30
---

# pimdir backend fixes owed before the pimdir stable release

From the pimdir stable-release review of 2026-10-30 and the plan's Phase A common items, the ones that need no new io-pimdir API.

- `copy_messages`, `move_messages` and `delete_messages` check every id before staging, skip an unknown one and return how many they found; they used to queue the first rows and fail on the first unknown id. Test `a_refile_or_delete_stages_the_ids_it_found`, failing on the old code with "Message `999` not found in `imap/INBOX`".
- A held item whose blob file is gone reads as `body-pending` (pimdir GUIDE §14) rather than an uncoded "Body blob missing". Test `a_body_gone_from_the_blob_tree_reads_as_pending`, failing on the old code with `left: None`.
- A flag change whose replacement set equals what the message shows stages nothing, the message still counted found; `pimdir flag add|set|remove` print it as unchanged, `queueId` `null`. Test `a_flag_change_the_message_shows_already_stages_nothing`, failing on the old code on the queued row. `a_queue_touching_another_mailbox_leaves_one_quiet` staged `\Seen` on a message that may carry it already; it now stages a keyword no seed carries, keeping its intent.
- `pimdir queue list` lists the mailbox's parked rows with why they parked (`parked` under `--json`), and `queue cancel` takes them. Test `a_parked_row_is_listed_and_cancelled` (a new view, so nothing to fail first).
- Every `pimdir` plain output and the shared `message add` and route confirmations end with a newline.
- Checked and locked: a store two accounts share is refused when `pimdir.account` is unset (`a_store_two_accounts_share_is_refused_without_an_account`); a create returns its link id and the queue view's ROW is the row id. The queue view's JSON envelope still carries `"id": ""`, the placeholder the rule excludes; left for a-queued-message-has-a-null-id.
- "pimdir SPEC" citations became STORAGE in src/config.rs, src/pimdir/client.rs and spec/backends.md; config.sample.toml's example account no longer names a real provider.
- CHANGELOG [Unreleased]: commit hashes dropped from the pimdir lines and the dependency bump, the role entry made true, and the fix of an unreleased `pimdir mailbox create` folded away.

Capabilities moved: **backends** (pimdir is a reader and a producer, never the owner; pimdir writes name their queue row; A pimdir refile or delete stages the ids it found; The pimdir queue view shows parked rows).
