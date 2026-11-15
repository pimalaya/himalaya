---
cairn: tasks
change: the-list-frame-is-shared
---

# Tasks

- [x] Cargo.toml: pimalaya-cli patched by path.
- [x] Every `Listing` and `ListingPage` from pimalaya-cli; src/listing.rs keeps `counts`; `Backend::as_str`.
- [x] Item errors last: the two lists through `Listing::errors`, the six writes through `write_item_errors`; byte-exact tests updated (envelope list, mailbox list, flag add, message delete, tests/exit.rs), the envelope one with two errors aligned.
- [x] `account capability` on the shared output, `CapabilityLevel` through `Support` and `StoreSupport`.
- [x] `account_capability` MCP tool, instructions, tests/mcp.rs.
- [x] CHANGELOG [Unreleased], MIGRATION.md (the cutting recipe).
- [x] Fold the delta into commands.md and mcp.md, write the log entry, set `landed`.
