---
cairn: change
id: the-kit-0-5-1-is-adopted
status: landed
created: 2026-11-15
---

# The kit 0.5.1 is adopted

## Why

The adoption line of the family's cli-alignment-2026-11-15 plan for Himalaya. pimalaya-cli 0.5.1 (plan lines L1, L4, L5) refuses a `repl` or `mcp` line inside a session itself, coded `usage-invalid` (owner ruling D3), answers a `--version` line once the binary's command enum carries its version, names the argument of a refused MCP call as the tool's input names it, and ships `output::SavedOutput`. Himalaya refused a nested session with its own uncoded arm (src/repl.rs), had no version line in a session, worded a refused call with clap's flag spelling (`--page-size`), and kept its own copy of `SavedOutput`.

## What

Move to pimalaya-cli 0.5.1, the lock changing that crate alone. Drop Himalaya's nested-session arm, now dead code; put `#[command(version, long_version = long_version!())]` on the command enum; turn clap's refusal of a tool call into the kit's wording through `tool.grammar.usage_error`; print the kit's `SavedOutput`. tests/repl.rs expects the refusal coded `usage-invalid` and gains a version line; tests/mcp.rs follows the new wording and the missing-argument refusal the kit makes before a command runs.
