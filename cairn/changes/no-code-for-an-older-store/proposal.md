---
cairn: change
id: no-code-for-an-older-store
status: landed
created: 2026-10-30
---

# No code for an older store

## Why

`store-older` (`ErrorCode::StoreOlder`, `PIMDIR_CODES` in src/error.rs, the JSON code table) relays a code io-pimdir no longer raises: io-pimdir refuses a store by version only when it is newer (`PimdirError::Version`, `store-newer`), reads an older one as it stands, and drops `store-older` from `PimdirError::code()` (io-pimdir, 2026-10-30). The code was never released (v2.2.1 has no code vocabulary), so removing it breaks no caller.

## What

- `ErrorCode::StoreOlder` and its `PIMDIR_CODES` entry go, and with them the `store-older` value of `himalaya-error`'s `code` schema.
- The code table, the pimdir store failure requirement and the CHANGELOG Added entry stop naming it.
