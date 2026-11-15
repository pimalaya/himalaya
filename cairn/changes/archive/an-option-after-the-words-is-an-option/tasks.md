---
cairn: tasks
change: an-option-after-the-words-is-an-option
---

# Tasks

- [x] src/pimdir/search.rs, count.rs, tag.rs, src/shared/message/arg.rs, src/sieve/script.rs, src/jmap/query.rs: drop `trailing_var_arg` and `allow_hyphen_values`, the help saying a word opening on a hyphen goes after `--`.
- [x] src/cli.rs tests: `pimdir tag --add x from:a --dry-run` sets `dry_run` with the query `from:a`, `pimdir search q -s 3 r --json` sets the page size and `--json` with the query `q r`, a negation after `--`, and the same for `message send`, `sieve put` and `jmap query`; each fails with the old attributes back.
- [x] tests/pimdir_search.rs `an_option_after_the_query_is_the_option`: a trailing `-s 2 --json` prints the JSON page, a trailing `--json` the JSON count, `-- -subject:3` negates.
- [x] Fold into cairn/spec/commands.md; CHANGELOG; MIGRATION.md; the log entry.
