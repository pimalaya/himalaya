---
cairn: change
id: a-server-refusal-codes-the-command
status: landed
created: 2026-11-15
---

# A server's refusal codes the whole command

## Why

Task H3 of the family's cli-alignment-2026-11-15 plan (findings F29, F30). Himalaya coded a whole-command failure only when it was an account refusal (src/error.rs `coded`), so an IMAP `NO [NOPERM]` or `[OVERQUOTA]` printed `code: null` though cairn/spec/commands.md promised `permission-denied` and `quota-exceeded`, a Graph 429 `MailboxConcurrency` printed `code: null` in CI, and a JMAP 403 or 429 too. A `message read` of an unknown id was uncoded on Maildir (``Maildir message locate failed: message 999 not found``), JMAP, Gmail, Graph and pimdir. Cardamum and Calendula read a code off the typed error of the protocol that answered, one reader per backend walked by `code(&Error)`; carillon and neverest map the IMAP codes.

## What

- One reader per protocol, walked over the error chain by `error::code` and handed to the failure object by `error::coded` through pimalaya-cli's `with_code`: IMAP `NOPERM` and `OVERQUOTA` (src/imap/client.rs), Gmail and Graph as their item errors code them (src/gmail/client.rs, src/msgraph/client.rs, which the item errors now share), JMAP HTTP 403 and 429 and the method error `forbidden` (src/jmap/error.rs).
- A JMAP SetError refusing the one write of a whole command coded by its type (`set_error`), in the unified mailbox writes, the import, the submission and the raw single-object writes.
- `coded` at every exit: the one-shot run and the MCP call already, the REPL line now; an `account check` row through `code`.
- `get_message` coding an unknown id `message-not-found` on Maildir, JMAP, Gmail, Graph and pimdir, a JMAP object that does not deserialize `message-unreadable`.
