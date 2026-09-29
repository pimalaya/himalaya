---
cairn: log
change: v1-backend-guard
landed: 2026-09-29
---

# v1 backend guard

Reported in #740: a v1 account kept on v2 failed with `No backend matching auto`, silent even at trace, its `backend.*` keys dropped as unknown.

`AccountConfig` gains `backend: Option<IgnoredAny>`, never serialized, so the v1 table is recognised without being modelled. `AccountConfig::no_backend_error` replaces "No backend matching `auto`", which read as if `auto` were a backend: it names the account, lists `Backend::COMPILED` (sieve excluded for the shared commands) under `auto`, names the missing block under a pinned backend, and adds the MIGRATION.md hint when the v1 table is present. A load-time warning was dropped: logging is off by default, so it reached nobody. Unknown keys in general stay tolerated for himalaya-tui.

## Capabilities moved

- [config](../spec/config.md): *A v1 account layout is named, not silently ignored* added.
