---
cairn: log
change: message-send-save-copy
landed: 2026-10-01
---

# `message.send.save-copy` names the mailbox a sent message is copied to

`message.send.save-copy`, global or per account, is the `--save` that `message send` and the composers with `--send` fall back on. It takes a mailbox name, alias or role; the v1 `true` reads as `sent` and `false` as no copy, and the `message` table accepts unknown keys so the rest of a v1 `[message]` table keeps loading. `--no-save` skips it for one call and conflicts with `--save`; `Account::resolve_save` holds the precedence.

`handler::apply` now sends before saving, for `--save` and `message add --send` alike: a failed send leaves no copy, and a save failing after the send errors with `Message sent, but saving a copy to <mailbox> failed`.

The wizard does not write the option: IMAP resolves no `sent` role until imap-special-use-aliases lands, and a generated `save-copy = "sent"` would fail every IMAP copy.

Capabilities moved: **config** (sent copies are configured per account), **commands** (saving a sent message follows the send).

Verified against a Maildir account and a scripted SMTP server: the configured copy, `--no-save`, `--save` overriding it, a refused send leaving no copy, a failing copy reporting the send, and `compose --send`.
