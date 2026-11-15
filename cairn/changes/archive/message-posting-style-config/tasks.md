---
cairn: tasks
change: message-posting-style-config
---

- [x] `PostingStyle` moves to config, `MessageQuoteConfig` under `message.reply` and `message.forward`
- [x] Merge into `Account`, resolved by `resolve_reply_posting_style` and `resolve_forward_posting_style`
- [x] `--posting-style` becomes optional on `message reply` and `message forward`
- [x] Test, config.sample.toml, CHANGELOG
- [x] Fold into spec/config.md, log
