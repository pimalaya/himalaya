---
cairn: change
id: stream-0-3-migration
status: landed
created: 2026-08-15
---

# pimalaya-stream 0.3 migration

## Why

Two reports, one failure: a command dies mid-exchange with a bare `Resource temporarily unavailable (os error 35)`, on Tencent Exmail during an IMAP `SORT` fallback ([#731]) and on a 260k-message Gmail account during `AUTHENTICATE` ([#732]). A blocking socket is not supposed to report `EAGAIN`, yet it surfaces anyway, macOS especially and the more readily the longer the exchange runs, and every protocol crate above treated it as the end of the session.

The fix belongs in the transport, not in any one protocol crate: io-imap, io-smtp, io-http, io-gmail, io-jmap, io-msgraph and io-pim-discovery all read and write through pimalaya-stream, and five of them armed a read deadline of their own and then treated its expiry as fatal, building the same failure in on purpose. pimalaya-stream 0.3 puts the retry in `Read` and `Write`, so every backend inherits it and none carries a loop.

Taking it is a breaking bump across the whole graph, which is what this change is: stream renamed `StreamStd` to `stream::Stream`, moved its connects onto per-transport options structs, flattened the `std` module away and replaced `proxy::dial` with `Proxy::connect`.

## What

Every crate in himalaya's graph that opens a socket moves to the new API: io-imap, io-smtp, io-http, io-gmail, io-jmap, io-msgraph and io-pim-discovery.

io-imap loses the retry layer it grew first, while that fix was still living at the protocol level: `ImapStream::read_some` / `read_response` / `write_bytes`, `ImapClientStd::timeout`, `ImapClientError::Timeout` and their backoff constants all go, leaving one policy in the tree instead of two stacked budgets. What stays is the empty-read guard, a real fix of its own: a peer hanging up mid-response used to resume the coroutine with an empty slice forever, and now fails with `UnexpectedEof`. io-imap gains `ImapStream::stop_retrying`, which the mailbox watch worker calls: it arms a read timeout precisely to be woken up, and a retrying stream would absorb the wakeup and leave the shutdown flag unchecked.

himalaya itself needs no code change. Its patch table points at the local checkouts, and `io-pim-discovery` moves from `0.5` to `0.6`, the local version.

## Scope / non-goals

No crate version is bumped and nothing is released, as in the 0.2 migration. The patch table points at local paths and must become git entries (or released versions) before this is committed, and the `pimalaya-stream = "0.2"` requirements across the graph become `0.3` when stream is released.

The `imap.timeout` config option is not part of this. A caller-facing budget is worth having, but the default minute is the fix for both reports, and a config field that only shortens it can come later.

io-oauth, mirador, neverest and the other consumers outside himalaya's graph are left alone.

[#731]: https://github.com/pimalaya/himalaya/issues/731
[#732]: https://github.com/pimalaya/himalaya/issues/732
