---
cairn: tasks
change: the-shared-repl-and-mcp
---

# Tasks

- [x] Tests first, seen failing against the old code: per-line `--json`, `-a`/`-b`/`--backend`/`-c` refused by name, `#` comments skipped, help on stderr in a JSON session (tests/repl.rs); a batch, a non-UTF-8 line and a request without `jsonrpc` refused while the server reads on, `initialize` instructions (tests/mcp.rs); the typed failure object for zero or two values and for a part tool printing no part (src/mcp.rs).
- [x] Cargo.toml: pimalaya-cli `repl` feature; shlex out.
- [x] src/repl.rs on `pimalaya_cli::repl::Repl`; the stdin guard out, call sites on `pimalaya_cli::session`.
- [x] src/mcp.rs on `mcp::serve`, `Grammar`, `ToolResult::printed`, `tool_output_schema`, with `instructions`; src/cli.rs passes no printer to `McpCommand::execute`.
- [x] Port the MCP unit tests to `Grammar`; the download test to the grammar's refusal of two ids.
- [x] `cargo fmt`, clippy on three feature sets, `cargo test --all-features`.
- [x] `json-schema -d` and `tools/list` (both gates) diffed before and after; an MCP session driven over stdio with the schemas validated.
- [x] README, CHANGELOG, src/main.rs header, src/session.rs doc.
- [x] Fold the delta into spec/repl.md and spec/mcp.md, log, set status landed.
