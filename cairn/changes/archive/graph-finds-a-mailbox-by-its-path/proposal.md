---
cairn: change
id: graph-finds-a-mailbox-by-its-path
status: landed
created: 2026-11-19
---

# Graph finds a mailbox by its path

## Why

Finding HIM-13 of the family's cli-family-analysis-2026-11-15 plan, its Graph half (the JMAP half landed as jmap-blobs-reuse-the-session on 2026-11-18): every one-shot Graph command naming a mailbox first sent the six well-known folder lookups through `$batch`, then listed the whole folder tree, every nested level and every page, to build an index it used once. An `envelope list` on a mailbox with 20 nested parents cost 23 round trips before its first message.

## What

A mailbox value is resolved along what it names: a folder id asked of Graph, a path walked only through the folders leading to it, a role or a well-known name asked as that well-known folder. The precedence stays the index's (id, path, role), as do the refusals (`mailbox-not-found`, several folders on one path). The resolved ids are kept for the client's life and forgotten after a folder write, as the index was. `mailbox list` keeps its whole walk and its `$batch` of roles.
