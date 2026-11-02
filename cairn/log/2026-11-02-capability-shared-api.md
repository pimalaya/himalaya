---
cairn: log
change: capability-shared-api
landed: 2026-11-02
---

# Capability-based shared API

The shared API is now the union of what the backends serve, held coherent by one meaning per verb, no longer an intersection. src/capability.rs names each shared capability in pimdir STORAGE Annex B's grammar without the domain word (`mailbox.list`, `envelope.search`, `envelope.search.<option>` per clause and sort key, `message.read|add|copy|move|remove|trash`, `attachment.list`, `flags.seen|answered|flagged|draft`, `submit`, `submit.copy`) and declares, in one table per backend compiled into every build, its level: `full`, `partial`, `server` (the dynamic tier) or `none`, each but `full` with a detail. The tables hold the shared verbs only; the inventory's protocol-only rows wait for their shared verb.

`Resolution` reads which blocks an account configures, nothing connecting, and is the one place the storage order lives (Maildir, m2dir, mbox, pimdir, JMAP, Gmail, Graph, IMAP, `--backend` as filter); `EmailClient` opens what it picked, and sending resolves on its own (the storage backend when it sends, else SMTP). Each shared command says what its arguments reach (each flag, each query option, `--save`, `--send`, `--seen`), checked in src/cli.rs `run_shared` against the kept client or the account's blocks before any connection, so a refusal resolves no credential and drops no kept client. A refusal carries the new code `capability-missing`, naming the account, the capability, the backend or the filter, and the detail; a query option fails as `search-unsupported`. The adapters check again at dispatch (`add_message`, `store_flags`, the save before a send).

What changes for a user: Gmail and Graph `flag add|set|remove answered|draft`, which reported the messages flagged and changed nothing, fail; Graph `message add` fails up front rather than with an uncoded error, and `--save` there before the send rather than after it; a JMAP send without `urn:ietf:params:jmap:submission` fails before the upload, leaving no draft. `message copy` on JMAP and Gmail is declared `partial`, one message in two mailboxes. Every verb a backend serves keeps its output.

The tables also write the `Backends:` footer of every shared leaf's `--help` and filter the MCP tools: `tools/list` leaves out a tool the account cannot serve at all (an SMTP-only account lists `message_send`, not `envelope_list`), and `envelope_search` ends its description with the options the backend answers fully, in part and not at all.

The shared tree is compiled in every build with a mail transport (`any(backend, feature = "smtp")`), dispatch alone gated: a build with SMTP alone parses every shared command and refuses the storage ones, and compiles again, which it had stopped doing at 158ee79 (unreleased). The error codes are declared in every build, so the 19 schemas `json-schema` writes for an SMTP-only build are byte-identical to the full build's. The protocol namespaces stay gated with their modules, and the manual pages, generated per command group by pimalaya-cli, do not carry the leaf footers yet.

With it, one-search-language landed its last items: the `envelope.search.*` options and the MCP description, and Gmail's date zone, a bare date being midnight PST to Gmail, so every bound is now Unix seconds of the UTC day (`after d` used to keep day d, `date d` two days).

pimdir: cairn/changes/mail-write-names-from-himalaya (draft, in the pimdir repository) proposes `collection.rename`, `collection.delete`, `mail.flags.important` and `mail.flags.junk` as Annex B rows, and argues the draft verbs and `submit.cancel` out.

Verified: 404 tests pass with all features. The 6 tests of tests/capability.rs and the 2 new MCP tests fail on the code before this change (run on a `git archive HEAD` copy). The Gmail translator tests pin the seconds. The resolver's unit tests cover the order, the one storage transport of the id-exchanging verbs and sending. Clippy is clean. The imap, maildir, gmail+smtp, jmap, pimdir, mbox, m2dir, msgraph and smtp builds compile without new warnings. The JMAP submission check has no test: no JMAP server is mocked here.

Spec: commands (opening rewritten; "Shared commands over a resolved transport" modified; Capability registry, An unserved verb is refused before connecting, A server-decided capability is refused once connected, Verbs exchanging ids share one transport, The shared command tree is the same in every build, Resolution changes no served verb added; `capability-missing` in the code table); backends (Shared operation set and Append gap modified; A copy on JMAP and Gmail is one message in two mailboxes, Gmail and Graph declare no answered or draft flag added); mcp (Tools are the classified shared API modified; The search tool says what the backend answers added); search (A clause a backend does not answer is refused with a code and Per-backend translation modified; What a backend searches is declared added).
