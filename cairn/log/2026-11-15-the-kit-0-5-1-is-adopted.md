---
cairn: log
change: the-kit-0-5-1-is-adopted
landed: 2026-11-15
---

# The kit 0.5.1 is adopted

The adoption line of the family's cli-alignment-2026-11-15 plan for Himalaya (plan lines L1, L4, L5, owner ruling D3).

## What landed

- pimalaya-cli 0.5.1 from crates.io. Lock: `cargo update -p pimalaya-cli` moved windows-sys 0.52.0 to 0.61.2 under errno, rustix, rustls-platform-verifier, tempfile and winapi-util; those lines were put back, so the lock changes pimalaya-cli alone.
- src/repl.rs: Himalaya's refusal of a `repl` or `mcp` line is gone, the kit refusing it before the line runs, coded `usage-invalid`, an alias or an inferred prefix too.
- src/cli.rs: the command enum carries the version and the long version, so `-V` and `--version` answer in a session, after a command too, as on the command line.
- src/mcp.rs: clap's refusal of a tool call goes through `Grammar::usage_error`, naming the property (``Invalid value `-1` for `page-size`: ...``). The kit also refuses a required argument left out before the command runs (``Missing argument `message-id` ``): `message_add` without `message`, which read stdin and failed `stdin-taken`, and `attachment_download` without `attachment-id`, which failed ``--stdout` takes exactly one attachment id``, now answer that.
- src/shared/output.rs: the kit's `output::SavedOutput`, JSON and text unchanged.
- Tests: tests/repl.rs `a_session_refuses_a_session_inside_it` expects `usage-invalid` (it fails on the uncoded arm), `a_version_line_prints_the_version`; tests/mcp.rs `a_refused_argument_is_a_usage_failure` checks the property wording and a missing `message-id`, `the_download_tool_takes_exactly_one_part_id` the missing `attachment-id`, and `a_failing_command_carries_its_code_and_the_server_goes_on` fails `attachment_list` on an unknown id (`message-not-found`) where `message_add` no longer reaches its command.

## Capabilities moved

- [repl](../spec/repl.md): *A session runs the CLI over one account*, *Help keeps JSON lines whole* and its scenario.
- [mcp](../spec/mcp.md): *A tool's input is the command's grammar* and the scenario *A required argument left out*.
- [commands](../spec/commands.md): *Raw message formats write bytes* names the kit's `SavedOutput`.
