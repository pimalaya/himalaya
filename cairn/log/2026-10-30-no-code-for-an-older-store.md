---
cairn: log
change: no-code-for-an-older-store
landed: 2026-10-30
---

# No code for an older store

`ErrorCode::StoreOlder` and its `PIMDIR_CODES` entry are gone from src/error.rs, so `himalaya-error`'s `code` schema no longer lists `store-older`: io-pimdir raises a version refusal only for a newer store and dropped the code. It was never released. The code table, the pimdir store failure requirement and the CHANGELOG Added entry no longer name it.

No test: a removed variant; `a_relayed_pimdir_code_keeps_io_pimdirs_spelling` still checks every relayed code against io-pimdir's spelling.

Capabilities moved: **commands** (A JSON error carries a stable code, modified), **backends** (A pimdir store failure carries its code, modified).
