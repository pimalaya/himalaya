---
cairn: log
change: an-account-says-what-it-serves
landed: 2026-11-03
---

# An account says what it serves

`himalaya account capability` (hidden aliases `capabilities`, `caps`) prints `{account, capabilities}`, one `{name, backend, level, detail}` row per shared capability in the order the shared leaves reach them (`capability::all`): the transport `Resolution` picks, `null` when none is configured, and its table's level and detail. `--backend` shows the account under that filter. It reads the configuration and the tables only; a `server` row stays `server`.

On a pimdir account, `PimdirClient::store_support` reads the writes from the store opened as a reader: an item write through io-pimdir's `capability::check` in each mail collection (`none` refused in all, `partial` refused in some or passing in part, with the refusal or the partial details), an intent through `performers`, the static row kept when no source declares anything. `Capability::annex_b_name` maps the names back onto Annex B, `mailbox.` read as `collection.`, the trash as `mail.message.move`. Not an MCP tool: the filtered `tools/list` already answers an agent.

Verified: 428 tests pass with the default features (new: tests/account_capability.rs, the resolved backends and the `--backend` filter on a Maildir synced from JMAP, nothing listening; the pimdir store's declared rows read as full or none and the Annex B names checked against io-pimdir's constants). The command did not exist before. The imap, maildir, gmail+smtp, jmap, pimdir, mbox, m2dir, smtp and sieve builds add no warning; clippy is clean.

Spec: commands ("An account lists what it serves" added).
