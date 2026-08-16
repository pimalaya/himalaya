---
cairn: tasks
change: compose-signature
---

# Tasks

- [x] src/config.rs: `signature` and `signature-delim` on `AccountConfig` and on the global `Config`, both in `RENDER_ORDER`; the account block's doc comment no longer calls them TUI-only, nothing per-account being so any more.
- [x] src/account/context.rs: the two fields on the runtime `Account`, merged account-over-global, plus `resolve_signature`, which lets `--signature-file` win by standing the config down, and `signature_delim`, which defaults to `DEFAULT_SIGNATURE_DELIM`.
- [x] src/shared/message/builder.rs: `BuilderArgs::signature_delim`, threaded into `compose_body`, which writes the separator verbatim instead of the hardcoded `"\n\n-- \n"`.
- [x] src/shared/message/{compose,reply,forward}.rs: the signature resolved through the account, the delimiter read off it, and the `--signature` help naming both.
- [x] himalaya-tui: `signature_block` assembling the same two keys before mml sees them, making its `signature-delim` live; its own change, spec and log entry.
- [x] Tests: the default block byte-identical to what was hardcoded, a custom and an empty delimiter, the `--signature` / `--signature-file` precedence, the default separator, and the TUI's block including the no-signature case.
- [x] config.sample.toml on both sides, and the CHANGELOG entries.
- [x] Fold the delta into [cairn/spec/config.md](../../spec/config.md) and [cairn/spec/commands.md](../../spec/commands.md); write [cairn/log/2026-08-16-compose-signature.md](../../log/2026-08-16-compose-signature.md).
