---
cairn: tasks
change: the-flag-glyphs-are-fixed
---

# Tasks

- [x] src/shared/envelope/list.rs: `UNSEEN_GLYPH`, `REPLIED_GLYPH`, `FLAGGED_GLYPH`, `ATTACHMENT_GLYPH` replace `FlagChars`; `format_flags` and `format_attachment` drop their glyph argument; src/shared/envelope/search.rs and src/pimdir/queue/list.rs follow
- [x] src/jmap/email: `EmailsChars` gone, the query and get commands and their dispatcher no longer take the account
- [x] src/account/context.rs: the glyph field, defaults, getters and merge gone
- [x] src/config.rs: `EnvelopeListTableConfig` gone, `envelope.list.table` read as `IgnoredAny`; the colour test becomes `envelope_table_is_ignored` (glyph and colour keys load, an `envelope.list` typo and a `mailbox` colour key are refused)
- [x] config.sample.toml: the four glyph lines removed
- [x] MIGRATION.md, CHANGELOG.md
- [x] Clippy over the 15 feature sets, the whole suite, the build without SMTP
- [x] Fold the delta into cairn/spec/config.md; log; status landed
