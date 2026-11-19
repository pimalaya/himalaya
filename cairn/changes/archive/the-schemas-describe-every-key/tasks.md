---
cairn: tasks
change: the-schemas-describe-every-key
---

# Tasks

- [x] B15 src/json_schema.rs: `ViewOrRaw`, `TemplateOrRoute`, `BytesOrSaved`, `HitsOrThreads`, `CandidatesOrChoice`.
- [x] B16 src/shared/message/read.rs, src/shared/message/delete.rs, src/maildir/message/add.rs, src/mbox/message/add.rs, src/maildir/flag/list.rs, src/mbox/flag/list.rs, src/jmap/query.rs.
- [x] C1 src/email/envelope.rs, src/email/mailbox.rs, src/email/flag.rs, src/imap/fetch.rs, src/imap/id.rs, src/shared/attachment/list.rs, src/shared/message/handler.rs, src/pimdir/mailbox/list.rs, src/pimdir/performer.rs.
- [x] C4 src/shared/message/read.rs on `crate::email::address::Address`.
- [x] B3 src/sieve/raw.rs, src/pimdir/performer.rs: `enum` through `schemars(extend)`.
- [x] Checked on a fresh `json-schema --dir` dump: no key without a description, no anyOf without one, no `None` nor intra-doc link; the registry tests pass. No regression test: descriptions only.
- [x] Fold into cairn/spec/commands.md (the `json-schema` paragraph); CHANGELOG (Changed); the log entry.
