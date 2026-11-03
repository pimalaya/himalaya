---
cairn: change
id: an-account-says-what-it-serves
status: landed
created: 2026-11-03
---

# An account says what it serves

## Why

The capability tables decide what each account can run, but nothing shows the result for an account: the `--help` footers list every backend, not the configured ones, and the MCP server's filtered `tools/list` reaches agents only. A frontend (himalaya-vim, himalaya-emacs, himalaya-tui) needs the same answer to hide an action before the user meets `capability-missing`, and a user needs it to see which backend serves what once an account configures several.

## What

`himalaya account capability` (hidden aliases `capabilities`, `caps`): one row per shared capability, in the order the shared leaves reach them, `{name, backend, level, detail}`, under `{account, capabilities}`. `backend` is the transport the resolver picks, `null` when none is configured; `level` and `detail` come from that backend's table. `--backend` shows the account under that filter.

It reads the configuration and the tables and connects nowhere, so it answers with no network and no credential. A `server` row stays `server`: only two capabilities are decided that way, and saying what the server must advertise is exact enough.

On a pimdir account the writes are read from the store, opened as a reader: an item write runs through io-pimdir's own gate (`capability::check`) in each of the account's mail collections, `none` when refused in all, `partial` when refused in some or passing in part; an intent (`collection.*`, `mail.submit`) is served when a source performs it (`performers`). A store whose sources declare nothing keeps the static row, as its gate refuses nothing. The mapping onto Annex B names is `Capability::annex_b_name`, `mailbox.` read as `collection.`, the trash a move.

## Decided

- **Under `account`, not a shared verb**: it describes the account, beside `account check`, and resolves no transport.
- **Separate from `account check`**, which connects and fails on a bad credential.
- **No MCP tool**: the filtered `tools/list` already gives an agent the answer, and a tool costs context on every turn.
