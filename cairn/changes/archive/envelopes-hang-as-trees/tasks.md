---
cairn: tasks
change: envelopes-hang-as-trees
---

# Tasks

- [x] pimalaya-cli `tree::Trees` (cli change a-list-hangs-as-a-tree, 15eea1b)
- [x] src/shared/envelope/list.rs: the tree under the mailbox, `format_detail` and `format_date` shared, `--recipient` printing `to `; tests for the tree, the recipients and control characters
- [x] src/imap/envelope/thread.rs: the envelopes fetched, one tree per thread, JSON entries with `from`, `date`, `flags`; tested on a nested thread and a missing parent
- [x] src/pimdir/thread.rs: the reply tree from `In-Reply-To`; tested on a chain and an orphan
- [x] src/config.rs, src/account/context.rs, config.sample.toml: `datetime-relative` gone, a test that it is refused
- [x] tests/repl.rs: the text line of the REPL test
- [x] CHANGELOG.md, MIGRATION.md (jq recipes), README.md
- [x] Fold into cairn/spec/commands.md and cairn/spec/config.md; log; status landed
- [x] Pin the pushed pimalaya-cli rev
