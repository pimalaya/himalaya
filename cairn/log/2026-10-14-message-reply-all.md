---
cairn: log
change: message-reply-all
landed: 2026-10-14
---

# `message reply --all`

`message reply --all` (`-A`) fills `Cc` with the other recipients of the source: its `To` and `Cc` without the account's `email`, without anyone already in `To`, each address once, display names kept. `To` is derived as before, and an explicit `--to` or `--cc` wins over the list it would derive ([#779](https://github.com/pimalaya/himalaya/issues/779)).

Capabilities moved: **commands** (reply to all recipients).

Verified on a Maildir account with `--json`, which prints the template without sending: the derived lists, `-A`, an explicit `--to` the `Cc` is then deduplicated against, and an explicit `--cc` replacing the derived one. `tests/message_reply.rs` runs the binary the same way.
