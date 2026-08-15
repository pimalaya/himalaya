---
cairn: log
change: stream-0-3-migration
landed: 2026-08-15
---

# pimalaya-stream 0.3 migration

himalaya and the seven io-* crates that open sockets moved to pimalaya-stream 0.3, which is what carries the fix for the two `Resource temporarily unavailable (os error 35)` reports ([#731], [#732]).

The failure was never himalaya's. A blocking socket is not supposed to report `EAGAIN`, yet one surfaces mid-exchange, macOS especially and the more readily the longer the exchange runs, and every protocol crate treated it as the end of the session. Stream 0.3 retries it inside `Read` and `Write`, for a minute per call before giving up with a `TimedOut` that names the budget, and arms a socket read deadline at connect time so the budget is enforceable against a server that simply goes silent. Nothing above chooses anything: the strategy is a field on the stream, and the `write_all` std builds on `write` inherits the whole thing.

Seven crates took the new API, all of it mechanical: `StreamStd` is `stream::Stream`, the `std` module is gone from the path, and each connect takes its transport's options struct. io-imap, io-smtp, io-http, io-gmail, io-jmap, io-msgraph and io-pim-discovery.

io-imap gave back more than it took. The retry had been written there first, while the fix was still living at the protocol level, so it arrived carrying a second implementation: `ImapStream::read_some` / `read_response` / `write_bytes`, `ImapClientStd::timeout`, `ImapClientError::Timeout` and the backoff constants, all now deleted rather than stacked on top of the transport's budget. Two things stayed. The empty-read guard is a real fix of its own, a peer hanging up mid-response having resumed the coroutine with an empty slice forever, and it now lives as one private `read_response` on the client covering all four loops plus the handshake. And `ImapStream::stop_retrying` is new, an empty provided method that the mailbox watch worker calls: that worker arms a 5s read timeout precisely to be woken up between IDLE keep-alives, and a retrying stream would spend its budget absorbing exactly that wakeup, leaving the shutdown flag unchecked until the server next spoke.

The graph also lost a duplicate on the way. io-gmail, io-jmap and io-msgraph required io-http `^0.3` while io-pim-discovery required `^0.4`, so two copies of io-http were resolved; none of the three uses io-http's client module, so the requirement moved to `0.4` and one copy remains.

himalaya's own code did not change. Its patch table now points at the eight local checkouts, and `io-pim-discovery` moved from `0.5` to `0.6`.

Capabilities moved: backends gained "Network transport resilience", the first spec statement about what a connection does when a socket is momentarily not ready.

Verified against a live Fastmail account (Cyrus 3.13): mailbox list, envelope list, envelope search with a sort clause, `imap id`, `imap status`, `imap sort`, `imap thread`, `message read` on a 185 B and on a 121 KiB message, `imap append` into Drafts followed by `imap store \Deleted` and `imap expunge`, `smtp raw NOOP`, `account check`, and live service discovery through io-pim-discovery's CLI. Unit suites pass in stream (8) and io-imap (220), and the build, fmt and clippy are clean.

Deliberately not done: no crate version is bumped and nothing is released, so the patch table still points at local paths and every `pimalaya-stream = "0.2"` requirement in the graph still reads 0.2. Both must be settled before this is committed and released. No `imap.timeout` config option, the default minute being what the two reports needed. JMAP, Gmail and Microsoft Graph were compiled but not exercised against a live account.

[#731]: https://github.com/pimalaya/himalaya/issues/731
[#732]: https://github.com/pimalaya/himalaya/issues/732
