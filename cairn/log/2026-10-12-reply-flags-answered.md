---
cairn: log
change: reply-flags-answered
landed: 2026-10-12
---

# A sent reply or forward flags its source

Requested in [#778]. `message reply --send` left the source as it was, so `envelope list` and every other client kept showing an answered message as one that was not. It now adds `answered` to the source once the send has succeeded, through the shared `store_flags`, and a flag failing at that point fails the command with an error stating that the message was sent, as a failing copy does.

`message forward --send` adds `forwarded` the same way. `$Forwarded` (RFC 5788) is a keyword, which a server may refuse where `\Answered` is a system flag it has to take, so a failure there is logged as a warning and the forward still succeeds. A reply or forward only printed or saved flags nothing.

Verified against Gmail over IMAP: the source kept its flags after a `reply` and a `forward` without `--send`, carried `\Answered` after `reply --send`, and `\Answered` and `$Forwarded` after `forward --send`. A server refusing the keyword was not exercised.

Capabilities moved: **commands** (A sent reply or forward flags its source).

[#778]: https://github.com/pimalaya/himalaya/issues/778
