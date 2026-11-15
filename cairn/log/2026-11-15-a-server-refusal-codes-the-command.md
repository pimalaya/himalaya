---
cairn: log
change: a-server-refusal-codes-the-command
landed: 2026-11-15
---

# A server's refusal codes the whole command

Task H3 of the family's cli-alignment-2026-11-15 plan (findings F29, F30): a whole command a server refused printed `code: null` (an IMAP `NOPERM`, a Graph 429 `MailboxConcurrency` in CI), and so did a read of an unknown id on Maildir, JMAP, Gmail, Graph and pimdir.

## What landed

- src/error.rs: `code` walks the chain through one reader per protocol, the account reader among them, and `coded` hands the code to the failure object through pimalaya-cli's `with_code`, as Cardamum and Calendula do. `unknown_message` codes a server's not-found answer `message-not-found`, its text kept in `sources`.
- Readers: src/imap/client.rs (`NOPERM` `permission-denied`, `OVERQUOTA` `quota-exceeded`, RFC 5530 §3, as carillon and neverest map them); src/gmail/client.rs and src/msgraph/client.rs, whose `refusal_of` the item errors now share; src/jmap/error.rs (HTTP 403 and 429 of a send, the session, a blob upload or download; the method error `forbidden`), and `set_error`, coding by its type the SetError of the one write a command makes (the unified mailbox create, rename and delete, the import, the submission, the raw `jmap mailbox create`, `identity create|update`, `submission create`, `email import`).
- src/repl.rs codes each line's failure; src/account/check.rs codes each row through `code`. The one-shot run and the MCP call already went through `coded`.
- `get_message`: Maildir, JMAP, Gmail, Graph and pimdir fail an unknown id `message-not-found`; a JMAP object that does not deserialize is `message-unreadable`.
- Tests: tests/refusal.rs, a scripted IMAP server refusing `CREATE` with `NOPERM` and `OVERQUOTA`, through a one-shot run, a REPL line and an MCP call (all three fail without the IMAP reader, the REPL one without its `coded`); unit tests of each reader; an unknown id read in tests/local_stores.rs (Maildir, mbox), tests/repl.rs, tests/mcp.rs, and the pimdir and JMAP backends.

## Capabilities moved

- [commands](../spec/commands.md): *A server's refusal codes the whole command*, and the `message-not-found`, `permission-denied` and `rate-limited` rows of the code table.
- [repl](../spec/repl.md): a command error is coded as a one-shot run codes it.
