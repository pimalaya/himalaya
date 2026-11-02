---
cairn: change
id: mcp-server
status: landed
created: 2026-11-02
---

# `himalaya mcp`: the shared API as MCP tools

## Why

Agents drive himalaya today by shelling out and parsing whatever comes back. The 2027 NLnet plan's Task 1, "Pimalaya CLI: REPL and MCP", gives them a typed door instead, and MOA needs MCP over mail (pimdir cairn/changes/search-milestones, decided 2026-10-27). The output side is ready: every payload is a JSON object with a stamped JSON Schema, failures carry stable codes, `--json` is one value per line (cairn/spec/commands.md, "The JSON contract is described and versioned"). MCP's `structuredContent` must be an object, which is exactly what `non_object_roots` already guards.

What was missing is the input side and a long-lived process. The process is the session of cairn/changes/repl-session; this change adds the MCP protocol on top of it.

## What

1. **Transport.** `himalaya mcp` runs an MCP server over stdio, JSON-RPC 2.0, one message per line: `initialize`, `ping`, `tools/list`, `tools/call`; notifications get no answer, an unknown method is `-32601`, a line that is not JSON `-32700`. It speaks revision 2025-06-18, the first carrying `outputSchema` and `structuredContent`, and answers it to every `initialize` (a client asking for a later one negotiates down). stdout carries protocol messages only; logs go to stderr. Stdio only: the pipe is the authorisation boundary, the argument ortie's REPL makes.
2. **Session.** Account and `--backend` are fixed at start, resolved once so a missing configuration fails the server rather than its first call. The kept clients, the rebuild after a failure and the stdin refusal are the REPL's (src/session.rs, `repl::claim_stdin`). Several accounts are several servers in the client's configuration.
3. **Tools.** One tool per leaf command of the shared API, named `<group>_<leaf>`, listed from a table (src/mcp.rs `TOOLS`) that classifies each as reading, writing or sending; a test fails when a shared leaf is neither in the table nor deliberately left out, so a new command is exposed only once someone decides what it may do. The protocol-specific APIs and the raw passthroughs are left out: they expose backend details the shared outputs were pruned of, and the raw ones return bytes, not objects.
4. **Input schema from the grammar.** Each tool's `inputSchema` is generated from its clap command after `Command::build`: an argument becomes a property named as the CLI spells it (the long flag, or the lowercased value name of a positional: `mailbox`, `page-size`, `message-ids`), its help the description, its possible values an `enum`, a flag a boolean, a repeatable argument an array, `required` from clap plus every positional taking at least one value, `additionalProperties: false`. A call is turned back into an argv (option values glued as `--flag=value`, positionals after `--`, so a value starting with a dash is never read as a flag) and parsed by the same clap command. Hand-written input structs were rejected: a second grammar drifts from the first.
5. **Refusals the caller reads.** An unknown key is refused naming the known ones, never ignored (MOA's agent once passed `query` to a tool expecting `text`, got the inbox back and took it for results; moa docs/decisions.md). That refusal, a clap parse failure and a command failure are all a tool result with `isError: true`, the text being the `himalaya-error` object with its code, rather than JSON-RPC errors, which a client often keeps from the model; only an unknown tool or malformed params are protocol errors (`-32602`).
6. **Output.** A result is the command's JSON value as `structuredContent`, with the same JSON as a text block. `outputSchema` is the payload's entry in the json-schema registry (`himalaya-<group>-<leaf>`) when its root is an object and the tool has one output shape: the composers (template or confirmation) and `message copy` and `move` (a confirmation line) declare none, and `message read` loses `raw`, which prints bytes instead of the read view. Output is captured by `McpPrinter`, which keeps each value printed; every exposed command prints exactly one under `--json` (checked by reading each `execute`: the two-print commands, `message read`, `parse` and `attachment download`, print on exclusive branches), and anything else is a tool error rather than a guess.
7. **Read-only by default.** Reading mail and sending it in one agent is an exfiltration path: a message the agent reads can tell it to forward the mailbox. Tool annotations (`readOnlyHint`, `destructiveHint`, `openWorldHint`) are set, but a client may ignore them, so the gate is the server's. Without flags only reading tools are listed; `--allow-write` adds the writing tools and the arguments that write (`save`, and `seen` on `message_read`, which flags the message); `--allow-send` adds `message_send` and the `send` argument of the composers and of `message_add`, and implies nothing else.
8. **No local files.** A tool never reads nor writes a path the caller picks: `attachment download` (writes files) and `message parse` (reads one) are left out, arguments with a path value hint or a `PATH` value name are not exposed (`--body-file`, `--attach`, `--signature-file`), and inside the server a message argument that spells an existing path is taken as the message itself (`mcp::paths_closed`, checked in src/shared/message/arg.rs). Without that, `message_add` given a path stored the file in the mailbox, readable back by any reading tool.
9. **No cargo feature.** The server adds no dependency, so nobody would toggle it off.

The generic part (a clap tree as tools, argv rebuilding, the capture printer, the JSON-RPC loop) is binary-agnostic and belongs in pimalaya-cli so cardamum and calendula get a server the same way; it lives in src/mcp.rs for now, as the REPL loop does, until pimalaya-cli takes both. A hand-rolled loop rather than the `rmcp` SDK: it is built on tokio, the family is blocking, and the surface needed is four methods.

## Later

- A `search` tool once himalaya reads io-pimdir's index (after Task 2): it wraps the `pimdir-search` contract, hits carry the address `message_read` takes, coverage says what was not searched. The embedding index's `mode` (pimdir cairn/changes/embedding-index) lands there.
- `attachment download` bounded to a configured directory.
- Notifications of new mail, resources, prompts, an HTTP transport, typed (integer) properties from clap's value parsers.

## Not in scope

The REPL itself: cairn/changes/repl-session.
