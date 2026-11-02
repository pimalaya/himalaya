---
cairn: log
change: mcp-server
landed: 2026-11-02
---

# `himalaya mcp`: the shared API as MCP tools

`himalaya mcp` is an MCP server (revision 2025-06-18) over stdio for one account, the second half of Task 1 in the 2027 NLnet plan ("Pimalaya CLI: REPL and MCP"), unreleased. It runs over the REPL's `Session` (cairn/log/2026-11-02-repl-session.md): clients kept for the server's lifetime, rebuilt after a failure, stdin claimed so no command reads its input there.

Tools are the shared API leaves listed in src/mcp.rs `TOOLS`, each classed reading, writing or sending; `every_shared_leaf_is_classified` fails when a shared leaf is neither listed nor deliberately left out. Each `inputSchema` is generated from the leaf's clap command after `Command::build`, properties named as the CLI spells them (`mailbox`, `page-size`, `message-ids`), `additionalProperties: false`; a call is rebuilt into an argv (`--flag=value`, positionals after `--`) and parsed by the same command. A first cut named properties by clap's field ids (`inner`, `max_width`), which is why the long flag or the positional's value name is used instead.

An unknown key, a parse failure and a command failure are all tool results with `isError: true` carrying the `himalaya-error` object, so the model reads them; only an unknown tool or malformed params are JSON-RPC errors. A result is the payload as `structuredContent` and as text; `outputSchema` comes from the registry when its root is an object and the tool has one output shape (not the composers, `message copy` or `move`; `message read` loses `raw`).

Without flags only reading tools are listed, without their writing arguments (`save`, and `seen` on `message_read`, which flags the message). `--allow-write` adds the writing tools and arguments, `--allow-send` the sending ones (`message_send`, `send` on the composers and `message_add`).

No tool touches a local file: `attachment download` and `message parse` are left out, path arguments (`--body-file`, `--attach`, `--signature-file`) are not exposed, and inside the server a message argument spelling an existing path is the message itself (`mcp::paths_closed` in src/shared/message/arg.rs). Before the guard, `message_add` given a path stored that file in the mailbox; `a_path_given_as_a_message_is_not_read_from_disk` checks the stored bytes and fails with the guard removed (a first version asserting on the read view passed either way, the file parsing as broken headers).

The generic part stays in src/mcp.rs until pimalaya-cli takes it with the REPL loop. No cargo feature: the server adds no dependency.

Verified: 374 tests pass with all features (5 in tests/mcp.rs); clippy clean on all features; `--no-default-features`, maildir-only and imap+smtp builds compile.

Spec: mcp (new: An MCP server over stdio, Tools are the classified shared API, A tool's input is the command's grammar, A tool's output is the command's payload, Writing and sending are opt-in, A tool reads and writes no local file); repl (`mcp` refused inside a session); commands (mcp among the meta commands).
