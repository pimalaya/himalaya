---
cairn: tasks
change: compose-from-identity
---

# Tasks

- [x] src/config.rs: `AccountConfig::email` and `AccountConfig::display_name`, aliased to `from` and `from-name`; the global `Config::display_name`, aliased to `from-name`; both keys added to `RENDER_ORDER` so a generated account leads with its identity.
- [x] src/account/context.rs: the two fields on the runtime `Account`, merged account-over-global, plus `resolve_from`, which resolves the `--from` override against them and hands back the address and the name apart.
- [x] src/shared/message/builder.rs: `BuilderArgs::from_name`, and `Address::new_address` instead of the bare `&str`, so `mail_builder` encodes the name.
- [x] src/shared/message/{compose,reply,forward}.rs: the default resolved through `account.resolve_from`, and the `--from` help naming it.
- [x] src/wizard/discover.rs: `prompted_email` reading the prompt back as an address, written onto the generated account; a URL's userinfo is not one.
- [x] Tests: the name-with-a-comma coming out quoted, the global name pairing with the account address, and `--from` dropping the configured name; the shapes `prompted_email` refuses.
- [x] himalaya-tui: `email` and `display-name` accepted as aliases of `from` and `from-name`, its own spec and log entry.
- [x] config.sample.toml and the CHANGELOG entry.
- [x] Fold the delta into [cairn/spec/config.md](../../spec/config.md), [cairn/spec/commands.md](../../spec/commands.md) and [cairn/spec/wizard.md](../../spec/wizard.md); write [cairn/log/2026-08-16-compose-from-identity.md](../../log/2026-08-16-compose-from-identity.md).
