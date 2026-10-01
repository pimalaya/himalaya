---
cairn: change
id: message-send-save-copy
status: landed
created: 2026-10-01
---

# `message.send.save-copy` names the mailbox a sent message is copied to

## Why

v2 dropped v1's `message.send.save-copy`, so a message sent over SMTP is kept nowhere unless every call passes `--save`. Each front-end (himalaya-emacs, himalaya-vim, himalaya-tui) would otherwise grow its own option, none of them knowing which backend an account sends through, while the need is per account: Gmail and Microsoft Graph file the sent message themselves, SMTP does not.

`--save` also appends before sending, so a send that fails leaves a copy of a message that never left.

## What

- `message.send.save-copy`, global or per account, is the `--save` a sending command falls back on. A mailbox name, alias or role; `true` stands for `sent`, `false` for none, a v1 boolean thereby reading as it used to.
- `message send`, `message compose`, `message reply` and `message forward` gain `--no-save`, conflicting with `--save`, to skip the configured copy for one send.
- Saving and sending, `--save` or `message add --send` alike, send first and save afterwards. A save failing after the send reports that the message was sent.
- The `message` table stays open to unknown keys, so the rest of a v1 `[message]` table keeps loading.

Left out: the wizard does not write the option. IMAP resolves no `sent` role until LIST `RETURN (SPECIAL-USE)` lands (imap-special-use-aliases), and the wizard writes no `mailbox.alias.sent`, so a generated `save-copy = "sent"` would fail on every IMAP send.
