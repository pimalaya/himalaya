---
cairn: tasks
change: stream-0-3-migration
---

# Tasks

- [x] Move io-imap onto `stream::Stream` and the connect options structs.
- [x] Strip io-imap's own retry layer, keeping the empty-read guard.
- [x] Add `ImapStream::stop_retrying` and call it from the mailbox watch worker.
- [x] Move io-smtp, io-http, io-gmail, io-jmap, io-msgraph and io-pim-discovery onto the new API.
- [x] Unify io-http on 0.4 across the graph, so one copy is resolved instead of two.
- [x] Point himalaya's patch table at the local checkouts and bump io-pim-discovery to 0.6.
- [x] Check every crate and himalaya itself, plus fmt and clippy.
- [x] Verify against a live Fastmail account: mailboxes, envelopes, search, sort, thread, read, append, store, expunge, SMTP and discovery.
- [x] Changelog entries in every touched crate and in himalaya.
