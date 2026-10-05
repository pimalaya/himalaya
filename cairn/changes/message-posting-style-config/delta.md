---
cairn: delta
change: message-posting-style-config
---

## ADDED Requirements

### Requirement: Posting styles are configured per account
`message.reply.posting-style` and `message.forward.posting-style`, global or per account, SHALL take `top`, `bottom` or `none` and SHALL be the posting style `message reply` and `message forward` use when `--posting-style` is not passed. With neither, the style SHALL be `top`.
