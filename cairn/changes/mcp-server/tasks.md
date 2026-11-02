---
cairn: tasks
change: mcp-server
---

# Tasks

Task 1 of the 2027 NLnet plan, "Pimalaya CLI: REPL and MCP", over cairn/changes/repl-session's session.

## himalaya (this repository)

- [x] Audit: every exposed command prints exactly one value under `--json` (two-print commands print on exclusive branches); anything else is a tool error
- [x] `Command::Mcp` (src/mcp.rs): stdio JSON-RPC loop, `initialize`, `ping`, `tools/list`, `tools/call`; refused inside a REPL
- [x] Tool table with read, write and send classes; a test that every shared leaf is classified or excluded
- [x] `inputSchema` from clap, CLI spellings, `additionalProperties: false`; argv rebuilt with glued values and `--`; unknown keys refused naming the known ones
- [x] `outputSchema` from the registry where the root is an object and the output has one shape
- [x] Failures as `isError` with the `himalaya-error` object
- [x] `--allow-write` and `--allow-send`, per tool and per argument (`save`, `seen`, `send`)
- [x] No local files: path arguments hidden, `attachment download` and `message parse` excluded, message paths taken as content (`mcp::paths_closed`)
- [x] Tests over stdio with a maildir fixture: gates, a call, an unknown argument refused, a failure carrying its code, a path not read (fails with the guard disabled)
- [x] README: the server and a client configuration; CHANGELOG
- [x] Fold into a new cairn/spec/mcp.md; log; land

## Later

- [ ] pimalaya-cli: the generic part (clap tree as tools, argv, capture printer, loop), with the REPL loop
- [ ] `search` tool over io-pimdir's index (after Task 2), with the embedding index's `mode`
- [ ] `attachment download` bounded to a configured directory
