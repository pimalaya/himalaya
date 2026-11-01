---
cairn: tasks
change: no-code-for-an-older-store
---

# Tasks

- [x] Drop `ErrorCode::StoreOlder` and its `PIMDIR_CODES` entry (src/error.rs); the JSON schema follows the enum.
- [x] CHANGELOG: drop `store-older` from the Added entry.
- [x] Fold the delta into [commands](../../spec/commands.md) and [backends](../../spec/backends.md); log; land.
