---
cairn: log
change: gmail-send-thread-id
landed: 2026-10-09
---

# Gmail messages send can join a thread

`gmail messages send` takes an optional `--thread-id <ID>`, sent as the message's `threadId`, matching the flag `gmail drafts create` and `gmail drafts update` already had. Without it the request is unchanged.

Capabilities moved: **commands** (Gmail writes can target a thread).
