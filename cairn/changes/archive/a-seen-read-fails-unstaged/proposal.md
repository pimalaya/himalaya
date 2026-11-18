---
cairn: change
id: a-seen-read-fails-unstaged
status: landed
created: 2026-11-18
---

# A seen read fails unstaged

## Why

Finding HIM-17 of the family's cli-family-analysis-2026-11-15 plan, its `--seen` point: on pimdir, `message read --seen` whose `\Seen` the store could not stage (a source declaring `mail.flags.seen` `none`, a store busy) logged a warning the default log level does not print and exited 0, the message read unseen. Maildir, mbox, Gmail, Graph and JMAP fail the command.

## What

- pimdir fails `message read --seen` when the flag cannot be staged, its cause kept (`unsupported`, `store-busy`, ...), as every other backend does.
- Breaking: exit 0 becomes 1 for that read.
