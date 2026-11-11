---
cairn: tasks
change: the-outputs-become-data
---

# Tasks

- [x] Pin pimalaya-cli through a path patch while 0.3.0 is unreleased
- [x] Shared receipts carry `queued`; copy, move, send and composers typed; flags `{raw, iana}`
- [x] Bytes through `Printer::out_bytes`: `message read --raw`, Gmail and Graph raw reads and attachments, JMAP export, a composer's message
- [x] `inReplyTo` a list, attachment `role`, `path: null`, designed outputs printing `null`
- [x] Envelope paging, Graph `$skip` token, `Paginated` from pimalaya-cli
- [x] `account check` report with `code`, exit 2
- [x] Gmail, Graph, JMAP, local stores, Sieve and SMTP receipts; Gmail settings writes print settings; Gmail list keys
- [x] IMAP completion outcome with refusals exiting 2; `imap raw` events; `imap fetch` addresses; `imap search` ids
- [x] `sieve raw` events, `smtp raw` code and text
- [x] pimdir `row`, flag `messages`, kebab-case values, `queue show` nulls, search and performer schemas
- [x] Registry: every command described, `OneOf`, events, check test
- [x] MCP and REPL handle events and partial outcomes
- [x] delta.md, spec fold, MIGRATION.md, CHANGELOG, log
- [x] ~~Pin pimalaya-cli by git rev once its commit lands~~ Superseded: pimalaya-cli 0.3 was released and required instead (8196891, 2026-11-03).
- [ ] Left for later: `imap fetch` streaming one event per message through `ImapMessageFetchStreamBatch`; MCP progress notifications for events; Graph `$search` pages (resumed by `$skiptoken`) refused by `--page-token`; raw-faithful passthrough of the Gmail and Graph `get` outputs still designed
