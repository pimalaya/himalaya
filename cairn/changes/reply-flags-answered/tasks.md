---
cairn: tasks
change: reply-flags-answered
---

- [x] `MessageReplyCommand::execute` adds `answered` to the source after a successful send
- [x] The error of a failing flag states that the message was sent
- [x] `MessageForwardCommand::execute` adds `forwarded` to the source after a successful send, a failure only logged
- [x] CHANGELOG
- [x] Fold into spec/commands.md, log
