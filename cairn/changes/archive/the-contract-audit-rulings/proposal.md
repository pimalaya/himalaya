---
cairn: change
id: the-contract-audit-rulings
status: landed
created: 2026-11-20
---

# The contract audit rulings

## Why

The family's CLI contract audit of 2026-11-20 (ledger cli-contract-audit-2026-11-20, findings HIM-X1 to HIM-X15) and the owner's rulings on its proposals: A1 (the shared check row carries `tls`, `null` over a cleartext connection), A2.5 (`account check` prints a list of accounts, a kit type), A4 (one `pimdir queue list` shape, `{actions, parked, errors}`, every pending kind, `queue show` the list row plus what a read carries, an unknown row `row-not-found`), A5 (no by-name refusal of a removed key, serde's `deny_unknown_fields` alone) and the himalaya section of the proposal. pimalaya-cli moved first (its change the-contract-audit-rulings): `account::AccountCheckOutput`, `BackendCheckTls` and `clap::meet_bare_invocation`.

## What

- `account check` prints pimalaya-cli's `AccountCheckOutput`, its `tls` `null` for a local backend and a cleartext connection; himalaya's own `AccountCheckReport`, `BackendCheckRow` and `TlsTrust` go.
- A bare run answers through pimalaya-cli's `clap::meet_bare_invocation`: `usage-invalid`, exit 2, under `--json`.
- A5: the `removed_key!` refusals go (`table`, `attachment`, `account`, `folder`, `envelope.list.table`), and the `m2dir` block and the v1 `backend` table, loaded and ignored, become unknown keys.
- X1: pimdir writes print the link id as `id` (`messageId` gone), the pimdir mailbox writes `id` too.
- X2: `message read --json` prints the `envelope list` row (its flags read in the same exchange) before `etag` and the view.
- X4: `page-token-invalid` on every list taking `--page-token`; `--page` beside it refused.
- X6: the pimdir mailbox writes on a store declaring nothing coded `capability-missing`.
- X7: under `himalaya mcp --allow-send` alone a send keeps no copy (`--no-save` fixed), but over a transport filing its own; `no-save` listed under the send gate. Checked at runtime first, a loopback SMTP responder showing the copy filed (`"saved":true`).
- X9: commands.md no longer lists the off-origin URL under `usage-invalid`.
- X10: the composer template carries `date` and `messageId`.
- X11: `attachment download` prints `Saved <size> bytes to <path>` per file.
- X12: one send receipt `{id, saved, sent, queued, errors}` for `message add`, `send` and the composers, a queued send `sent: false`; `message delete` prints `ids`; the raw Maildir writes print the folder's path as `id`, `maildir list` rows gain `id`.
- X13 (A4): `pimdir queue list` lists every pending row.
- X14: `imap raw` checks its tags before the session opens.
- X15: exit 3 named in the help of the partial commands; a `search-invalid` failure one line under `--json`.
