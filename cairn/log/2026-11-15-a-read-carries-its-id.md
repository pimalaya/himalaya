---
cairn: log
change: a-read-carries-its-id
landed: 2026-11-15
---

# A read carries its id

Task H8 of the family's cli-alignment-2026-11-15 plan (finding F9, owner ruling D7), with pimalaya-cli's rule that a read carries `id` and `etag`, `etag` `null` where the backend versions no item.

## What landed

- src/shared/message/read.rs: `MessageReadOutput` is `{id, etag}` beside the flattened `MessageReadView`; `id` the id the command took, `etag` `null`. `message parse` prints the view alone. The text output is unchanged.
- Tests through the binary: tests/local_stores.rs (Maildir, mbox), tests/repl.rs, tests/mcp.rs; tests/message_parse.rs compares the parse with the read less `id` and `etag`.
- CHANGELOG; the MIGRATION.md bullet on the read view.

## Capabilities moved

- [commands](../spec/commands.md): *A read message is a designed view*, *A message can be read without an account*.
