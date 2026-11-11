---
cairn: tasks
change: wide-lists-print-as-records
---

# Tasks

- [x] Cargo.toml: pimalaya-cli patched to the ../cli checkout carrying `records`, until it is pinned
- [x] src/shared/envelope/list.rs: `Records`, `format_state` replacing `format_flags`, `format_attachment` and the four glyph constants, `format_addresses` printing `Name <email>`, `with_attachment` gone; envelope search follows; byte-exact test rewritten
- [x] src/jmap/email/query.rs (`answered` added from `$answered`), src/msgraph/messages/list.rs (`flagged` and `attachment` added)
- [x] src/pimdir/index.rs (hits for search and thread, `PimdirMailCoverage::write_under`), src/pimdir/search.rs (and `--threads`; a test pins a hit and the blank line before the coverage), src/pimdir/tag.rs, src/pimdir/queue/list.rs (queued messages; parked rows stay columns)
- [x] src/shared/attachment/list.rs and download.rs (`with_inline`, `with_path` gone; byte-exact test added), src/pimdir/attachment/list.rs, src/msgraph/attachments/list.rs, src/msgraph/mail_folders/list.rs
- [x] src/gmail/settings/filters/list.rs, src/gmail/history/list.rs
- [x] Audit of the other `Columns::new`: kept as columns
- [x] tests/repl.rs: the text answer of `envelope list` is a record
- [x] Doc comments: main.rs, the DATE column wording, `--has-attachment`
- [x] README.md, MIGRATION.md, CHANGELOG.md, config.sample.toml
- [x] fmt, clippy over the 15 feature sets, the whole suite, the build without SMTP; `json-schema` output and `--json` compared with the previous build
- [x] Fold the delta into cairn/spec/commands.md, config.md and backends.md; log; status landed
