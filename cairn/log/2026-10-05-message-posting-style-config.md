---
cairn: log
change: message-posting-style-config
landed: 2026-10-05
---

# `message.reply.posting-style` and `message.forward.posting-style`

`message.reply.posting-style` and `message.forward.posting-style`, global or per account, set the posting style (`top`, `bottom` or `none`) used when `--posting-style` is not passed, `top` remaining the default ([#772](https://github.com/pimalaya/himalaya/issues/772)). `PostingStyle` moved from the message builder to the config so the schema compiles under any feature subset.

Capabilities moved: **config** (posting styles are configured per account).

Verified against a Maildir account: a global `bottom`, a per-account `none` overriding it, `-P top` overriding both, and `message forward` keeping `top` absent its own key.
