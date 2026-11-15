---
cairn: change
id: the-shared-repl-and-mcp
status: landed
created: 2026-11-11
---

# The shared REPL and MCP

## Why

pimalaya-cli now carries the REPL loop (`repl::Repl`), the stdin guard of a session (`session::claim_stdin`, `check_stdin`, `in_session`) and the MCP protocol core (`mcp::read`, `serve`, `ToolResult`, `grammar::Grammar`, `tool_output_schema`), extracted from Himalaya and Ortie and already run by Ortie. Himalaya still holds its own copies: a hand-rolled loop that adds a blank line after every text result, cannot take `--json` on one line, answers a `#` comment with a parse error, prints help on stdout in a JSON session and answers `-a` with clap's generic error; a JSON-RPC reader that accepts a request without `jsonrpc`, answers a batch with nothing and ends the server on a line that is not UTF-8; and a tool result that turns a command printing no value or several into untyped text, its count off by one.

## What

- src/repl.rs runs `Repl::new("himalaya", json)` with `-b`/`--backend` added to the fixed flags, the closure refusing `repl` and `mcp` and running every other command over the kept `Session`. `ReplLine`, the loop and the blank line after text results go.
- The stdin guard goes from src/repl.rs; its five call sites (raw commands, `message parse`, the message argument, `jmap query`, the Sieve script argument) call `pimalaya_cli::session::check_stdin(ErrorCode::StdinTaken)`, the composers' body `in_session`.
- src/mcp.rs serves through `mcp::serve` (which claims stdin), with `ServerInfo` carrying instructions. Tool input schemas come from a `Grammar` chain (`retain` for the gates and `without`, `fixed`, `one`), the server-only arguments inserted into `input_schema()`, the annotations read off `names()`, the output schemas through `tool_output_schema`. A call builds `ToolResult::printed::<ErrorCode>`, the download tool moving the part's bytes into an embedded resource on that result. `McpServer::answer`, `failure`, `McpPrinter`, `McpToolArg`, `argv`, `exposes`, `scalar` and `fixes` go.
- pimalaya-cli gains its `repl` feature; Himalaya's direct shlex dependency goes.
- README, CHANGELOG (the unreleased REPL and MCP entries amended), src/main.rs header, src/session.rs doc.

## Scope / non-goals

The tool table, the gates, the server-only `attach-data`, the attachment cap and the part URI stay as they are: the `tools/list` definitions and the `json-schema` output are unchanged. pimalaya-cli and its specs are not touched.
