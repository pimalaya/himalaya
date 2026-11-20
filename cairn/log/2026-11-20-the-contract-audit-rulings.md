---
cairn: log
change: the-contract-audit-rulings
landed: 2026-11-20
---

# The contract audit rulings

Himalaya's share of the family's CLI contract audit of 2026-11-20 (findings HIM-X1 to HIM-X15) and of the owner's rulings A1, A2.5, A4 and A5, over pimalaya-cli's unreleased account check output and bare-invocation answer, reached through a path patch.

## What landed

- `account check` prints `{accounts: [{account, backends}]}`, each row `{backend, ok, code, error, tls}`, `tls` `null` for a local backend and a cleartext connection (an `imap://` or `smtp://` server without STARTTLS, `sieve://` with it off, an `http` JMAP server).
- A bare `himalaya --json` fails `usage-invalid`, exit 2.
- The removed tables and the `m2dir` and `backend` blocks are unknown keys, refused by serde alone.
- `message read --json` prints the `envelope list` row; every backend's read returns the flags with the bytes (IMAP `FLAGS` in the same `FETCH`, JMAP `keywords` in the same `Email/get`, Graph's flag properties in the `$select` it already sent).
- One send receipt for `message add`, `send` and the composers, a queued send `sent: false`; `message delete` prints `ids`; the raw Maildir writes print the folder's path as `id`.
- The pimdir writes print `id`; `pimdir queue list` prints `{actions, parked, errors, refused}`, every pending kind; `pimdir queue show` prints the list row then `state`, `appliedAt`, `refusals`, an unknown row `row-not-found`.
- `page-token-invalid` on every list taking `--page-token`, `--page` beside it refused; `capability-missing` on the pimdir mailbox writes of a store declaring nothing; `imap raw` checks its tags before connecting.
- Under `himalaya mcp --allow-send` alone a send keeps no copy. Seen at runtime first: with `message.send.save-copy = "Sent"`, a `message_send` call under `--allow-send` printed `"saved":true` and filed the copy.
- The composer template carries `date` and `messageId`; `attachment download` prints `Saved <size> bytes to <path>`; a `search-invalid` failure is one line under `--json`; the help of the partial commands names exit 3.

## Capabilities moved

- [commands](../spec/commands.md): *account check covers every declared backend*, *A query error is drawn without colour*, *The exit status tells the outcome*, *The JSON contract is described and versioned*, *Composers default the From header*, *A read message is a designed view*, *Downloaded attachments never replace an existing entry*, *A queued write says it is queued*, *The raw Maildir folder commands guard the store*, *A write prints what it acted on*, *Listings resume from their cursor*, *A value a protocol command refuses is refused with the command line*.
- [backends](../spec/backends.md): *The pimdir subcommand reads and retracts the queue*, *pimdir writes name their queue row*, *The pimdir queue view shows parked rows*, *pimdir creates a mailbox by queueing an intent*.
- [config](../spec/config.md): *Unknown keys are refused*, *No colour is configured*; *A missing backend is named* and *No m2dir store is read* replace the two requirements naming the v1 `backend` table and the `m2dir` block.
- [mcp](../spec/mcp.md): *Writing and sending are opt-in*.

## Left

- A pending row that does not decode makes its whole mailbox one item error: io-pimdir reads a collection's pending rows whole, its raw rows private.
- The path patch on ../cli becomes a release pin before commit; the `$id` stays `v2` until the 3.0.0 bump (HIM-X8, v3-release).
