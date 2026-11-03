---
cairn: log
change: the-union-gains-its-verbs
landed: 2026-11-03
---

# The union gains its verbs

The shared API now carries the inventory rows that keep one meaning across their backends, each a capability in src/capability.rs, a `--help` footer, a schema and an MCP tool.

`mailbox create <NAME> [--parent <MAILBOX>]`, `mailbox rename <MAILBOX> <NAME>` and `mailbox delete <MAILBOX>` (`mailbox.create`, `mailbox.rename`, `mailbox.delete`) run on every backend: a name is one level, a rename keeps the parent (pimdir's `collection.rename`, settling capability-shared-api's open question for `rename`, reparenting left to a later `mailbox move`), a delete takes the messages. IMAP joins levels with the delimiter its `LIST` reports, Gmail and the local stores with `/`; Gmail renames through `labels.patch`. Partial: Graph creates under a parent in two requests (io-msgraph has no child-folder creation), m2dir refuses `--parent` (flat folders), JMAP's delete keeps a message also in another mailbox, Gmail's leaves the messages under their other labels, Graph's takes the folders held. m2dir has no rename. On pimdir each is queued, `id` null. A write drops the cached mailbox index of JMAP, Gmail and Graph (`forget_mailboxes`).

`flag … --flag important|junk` (`flags.important`, `flags.junk`): JMAP, m2dir, pimdir full; Gmail `important` full, `junk` partial (the `SPAM` label moves the message); Graph `important` partial (`importance: high`), `junk` none; Maildir and mbox partial (a keyword other readers may not see); IMAP `server`: `store_flags` now refuses, after the `SELECT`, a keyword the mailbox's `PERMANENTFLAGS` lists neither by name nor by `\*`, under `capability-missing`.

`vacation get|set` (`vacation`): `{enabled, from, to, subject, text, html}`, dates RFC 3339 UTC; `set` changes what it is given and prints the result. JMAP `server` (`urn:ietf:params:jmap:vacationresponse`), Gmail full (epoch milliseconds, settings read first). An account feature resolves on its own: `Resolution::features`, the first configured backend serving it, so a Maildir synced from JMAP reaches JMAP; dispatch runs over the session's `jmap` or `gmail` client. IMAP and the local stores refuse it naming `himalaya sieve` or the provider; Graph until io-msgraph has `mailboxSettings`. No `--no-from`/`--no-to`: io-jmap cannot send the `null`. MCP lists `vacation_set` under `--allow-send`.

Verified: 424 tests pass with the default features (new: the resolver of the auto-reply, the path helpers, the `PERMANENTFLAGS` decision, the vacation dates, the queued creation output, 3 refusals in tests/capability.rs, the Maildir create, rename and delete in tests/mailbox.rs, the MCP listing). The new verbs did not exist before, so their tests fail on the old code by construction. The imap, maildir, gmail+smtp, jmap, pimdir, mbox, m2dir, msgraph and smtp builds compile without new warnings; clippy is clean. No live account was touched: the JMAP, Gmail and Graph adapters are compiled and typed, not exercised.

Spec: commands (opening; "A server-decided capability is refused once connected", "Verbs exchanging ids share one transport", "A queued write says it is queued" modified; "An account feature resolves on its own", "Shared mailbox writes", "The important and junk flags are shared", "The auto-reply is shared" added); backends ("Shared operation set" modified; "An IMAP keyword is kept or refused" added); mcp ("Writing and sending are opt-in" modified).
