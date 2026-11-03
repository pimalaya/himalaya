---
cairn: log
change: the-outputs-become-data
landed: 2026-11-03
---

# The outputs become data

pimalaya-cli's every-output-is-data removed `Message`, which broke the 111 `{"message": ...}` sites the JSON review of 2026-11-03 had counted at 2f49f86, 56 of them hiding data. Each now prints a typed value; its sentence stayed as the text form.

Shared commands: receipts serialize `queued`, `message copy|move` print `{count, queued}`, `message send` and the composers `{id, saved, sent, queued}` (the saved id the code used to drop), flag writes their `{raw, iana}` flags, `message read --raw` and the other byte writers go through `Printer::out_bytes`, `inReplyTo` is a list, attachment rows carry `role` and `path: null`, envelope listings print `nextPage` and take `--page-token`, `account check` prints every row with its `code` and exits 2 through `PartialOutcome`.

Protocol namespaces: receipts `{id}`/`{ids}` in a new shared::receipt module for Gmail, Graph, JMAP, the local stores and Sieve; Gmail settings writes print the settings through a `From` impl moved out of each `get`; the Gmail list payloads flattened under the provider's keys. IMAP gained imap::outcome: every typed command prints `ImapOutcome` (`status` beside its data, or `{status, text}` read from the `No`/`Bad` variant of io-imap's per-command error and exiting 2). `imap raw` splits the reply, literals kept whole, into `untagged`, `continuation` and `unparsed` events and the completions. `sieve raw` and `smtp raw` print their completions split. pimdir rows are `row`, its flag output `{messages}`, its values kebab-case, `queue show` keys `null` rather than omitted.

The registry builds on `JsonSchemaRegistry::new`; a schema-only `OneOf` describes the commands printing one of two shapes (`message read`, the composers, the raw `get`s, the attachment `get`s, `pimdir search` and `performer`). The new check test listed 110 undescribed commands and one dangling key (`himalaya-pimdir-search-threads`) before the entries were added. MCP collects events as text content and keeps a partial outcome's result; the REPL prints no failure object for one.

Verified: 392 unit tests and the integration tests pass with all features; clippy is clean with all features and with each backend feature alone (the sieve-only, msgraph-only and no-feature builds keep their earlier dead-code warnings in files this change did not touch). `json-schema --dir` writes 216 schemas. On a scratch maildir account: `message add`, `envelope list -s 1` (`"nextPage":"2"`, then `--page-token 2`), `flag add`, `message read --raw --json`, `message copy`, `maildir create` printed the new shapes, and `account check` on a missing root printed its row and exited 2. No live account was touched.

Spec: commands (5 requirements added, 11 modified), backends (4 modified), search (1 modified), mcp (1 modified).

Left: pin pimalaya-cli by git revision once its commit lands (a path patch now); `imap fetch` events through `ImapMessageFetchStreamBatch`; MCP progress notifications; Graph `$search` pages resumable; the designed Gmail and Graph `get` outputs left as they were rather than passed through raw.
