---
cairn: tasks
change: the-mcp-mbox-stays-in-its-root
---

# Tasks

- [x] Integration test `an_mbox_outside_the_root_is_refused_by_the_server` (tests/mcp.rs): `envelope_list` listed the outside file on the old code; now both calls are tool errors, the file unchanged, a mailbox of the root still listed and the CLI still opening the path.
- [x] The refusal in `resolve_mbox`.
- [x] Fold into cairn/spec/backends.md and cairn/spec/mcp.md; CHANGELOG; the log entry.
