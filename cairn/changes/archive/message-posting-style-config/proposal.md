---
cairn: change
id: message-posting-style-config
status: landed
created: 2026-10-05
---

# `message.reply.posting-style` and `message.forward.posting-style`

## Why

v2 dropped v1's `template.reply.posting-style`, leaving only `--posting-style`, which defaults to `top`. A bottom-poster has to pass the flag on every reply ([#772](https://github.com/pimalaya/himalaya/issues/772)).

## What

- `message.reply.posting-style` and `message.forward.posting-style`, global or per account, taking `top`, `bottom` or `none`.
- `--posting-style` wins over them, `top` answering when nothing is set; `Account::resolve_reply_posting_style` and `resolve_forward_posting_style` hold the precedence.

Left out: the v1 `template.*` keys are not read, and v1's `interleaved` has no v2 equivalent (interleaving is left to the writer with `none`).
