---
cairn: log
change: the-flag-glyphs-are-fixed
landed: 2026-11-11
---

# The flag glyphs are fixed

The owner's decision of 2026-11-11: remove the glyph keys from the CLI without making their block strict.

- The FLAGS and ATT glyphs are constants of src/shared/envelope/list.rs (`*`, `R`, `!`, `@`), read by `envelope list|search`, `pimdir queue list` and `jmap email query|get`; `FlagChars`, `EmailsChars`, the account's glyph field, defaults, getters and merge are gone, and the two JMAP commands no longer take the account.
- `envelope.list.table` is read as `IgnoredAny`: its v2 glyph and colour keys load and are ignored, while `envelope.list` still refuses a key of its own it does not know. `mailbox` colour keys stay refused. himalaya-tui reads none of these keys today and tolerates them either way.
- config.sample.toml loses its four glyph lines; MIGRATION.md now names `mailbox` as the one place a v2 colour key fails.
- Capability moved: config (requirement "No colour is configured", now also no glyph, with two scenarios).

Verified: clippy over the 15 feature sets, the whole suite with all features, the build without SMTP; a scratch Maildir account setting `unseen-char = "N"`, `flagged-char = "F"`, `attachment-char = "A"` and a colour key listed `*!`, `R` and `@`, and `envelope.list.datetime-fnt` was refused by name.
