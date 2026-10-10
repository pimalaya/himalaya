---
cairn: change
id: attachment-exclusive-create
status: landed
created: 2026-10-10
---

# Create attachment files exclusively

## Why
Checking existence before writing follows dangling symlinks and races with other writers. Exhausting the numbered names returns an occupied path and overwrites its content.

## What
Atomically create each candidate with `create_new`, retry occupied names, and fail when all candidates are occupied. Keep decoded bytes, filename sanitization and numbered naming unchanged. The user approved these security fixes and fork-first validation before implementation on 2026-10-10.
