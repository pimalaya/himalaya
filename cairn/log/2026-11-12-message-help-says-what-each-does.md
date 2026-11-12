---
cairn: log
change: message-help-says-what-each-does
landed: 2026-11-12
---

# The message help says what each subcommand does

`himalaya message --help` lists its subcommands by purpose, `read` and `parse`, the builders `compose`, `reply` and `forward`, the raw `send` and `add`, then `copy`, `move` and `delete`, and each help says what it does: the builders print the message unless `--save` or `--send`, a sent message's copy going where `message.send.save-copy` puts it; `send` and `add` take a raw message as it is; `parse` reads a file or its contents on stdin. The owner asked for it on 2026-11-12. The help had `--send` needing `--save` for a copy, `message send` routed through SMTP or JMAP alone and `message read <id>` piped into a composer; it now says what the code does. No capability moved.
