---
cairn: tasks
change: a-refused-line-exits-two
---

# Tasks

- [x] Pin pimalaya-cli at eb379626e01e31bed11cbe7f06c252262e810279
- [x] src/main.rs: `clap::parse` in place of `Cli::parse`
- [x] src/mcp.rs: the argv clap parse through `usage_error`
- [x] Renumber the partial outcomes to 3 in the doc comments, the `account check` unit test and tests/common.rs
- [x] Tests: tests/exit.rs (a refused line with and without `--json`, `--json` after `--`, a failed check exiting 3), tests/mcp.rs (an undeclared argument and a value clap refuses coded `usage-invalid`), tests/repl.rs (a refused line coded); each failing against the old pin or the old src/mcp.rs
- [x] Spec: commands (exit statuses, code table, JSON contract, raw passthrough, account check, IMAP completion), repl, mcp, packaging; README FAQ; CHANGELOG; MIGRATION
- [x] fmt, clippy per feature set, tests
