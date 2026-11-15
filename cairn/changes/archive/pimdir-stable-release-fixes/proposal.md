---
cairn: change
id: pimdir-stable-release-fixes
status: landed
created: 2026-10-30
---

# pimdir backend fixes owed before the pimdir stable release

## Why

The pimdir stable-release review of 2026-10-30 (report-himalaya.md, should-fix 3, 7, 8, 11, 13 and release blocker 5) and the plan's Phase A common items found these in the pimdir backend, each verified on a scratch store unless noted. None needs the coming io-pimdir API.

- **Partial writes reported as failures.** `delete_messages` and `refile` stage a row per id and bail on the first unknown one (src/pimdir/backend.rs:688-724): `message copy --from imap/INBOX --to imap/Trash 3 999` exited 1 with "Message `999` not found" after queueing a `copy` for 3. cairn/spec/commands.md ("A command given message ids says how many it found") says such a call succeeds and reports how many it found; `refile` also returns `ids.len()` rather than the count found.
- **A missing blob is a hard error.** A held item whose blob file is gone reads as `{"error":"Body blob missing ..."}` with no code (src/pimdir/backend.rs:379-381). pimdir GUIDE §14 says a blob gone under a purge is not hydrated and not an error: it is `body-pending`.
- **No-op flag changes queue rows.** `flag add -f seen 6` on a message showing `\Seen` queued a second identical `set-flags` (src/pimdir/backend.rs:435-460).
- **Plain outputs without a trailing newline**: `pimdir queue show`, `queue cancel`, `performer`, `message add`, `message send`, `mailbox create`, and the shared `message add` and `MessageRouteOutput` ("...saved and sentexit=0").
- **Parked actions are invisible.** A parked row (an `add` whose link id the mailbox already holds, a send its performer refused) leaves `pimdir queue list` and shows nowhere but `pimdir queue show <ROW>`, for a row id the user may no longer have.
- **Docs and changelog.** CHANGELOG.md:26 claims role-name addressing (true once a-pimdir-role-names-its-mailbox lands) with a commit hash, CHANGELOG.md:49 names io-pimdir `189db40` (the pin moved), and the [Unreleased] section carries io-pimdir commit hashes where it should be a net diff. Code and spec cite "pimdir SPEC", a document since split into STORAGE, SYNC and SEARCH. config.sample.toml names an example account after a real provider.

Two plan items hold already, checked rather than changed: a store several accounts share is refused when `pimdir.account` is unset (src/pimdir/client.rs `resolve_account`), and a queued creation never carries a placeholder id (spec/backends.md "A queued creation is reported, not listed", the 2026-08-27 pimdir-queue-visibility log). Both get a test locking them.

## What

- `delete_messages` and `refile` check every id first, skip and log the unknown ones, stage the found ones and return their count; none found stages nothing and the shared command fails as on every backend.
- A held item whose blob is missing fails with `body-pending`, saying the body is not local.
- `stage_flags` skips a message whose flags already equal the replacement set: nothing is queued for it, it still counts as found, and `pimdir flag add|set|remove` print it as unchanged (`queueId` `null`).
- Every plain output above ends with a newline.
- `pimdir queue list` lists the mailbox's parked rows under the queued ones, any action kind, with the row id, the kind, when it was queued and why it parked; `--json` carries them as `parked`. `queue cancel` takes a parked row as it takes a pending one.
- "pimdir SPEC" citations become STORAGE (or SYNC) with their section; config.sample.toml's example account becomes a neutral name.
- CHANGELOG [Unreleased] rewritten as a net diff for the pimdir lines, without commit hashes.
