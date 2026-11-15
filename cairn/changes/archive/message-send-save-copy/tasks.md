---
cairn: tasks
change: message-send-save-copy
---

- [x] `MessageConfig`/`MessageSendConfig` with `save-copy` (string or boolean), global and per account
- [x] Merge into `Account`, resolved by `Account::resolve_save`
- [x] `--no-save` on `message send`, `compose`, `reply`, `forward`
- [x] `handler::apply` sends before saving
- [x] Tests, config.sample.toml, CHANGELOG
- [x] Fold into spec/config.md and spec/commands.md, log
