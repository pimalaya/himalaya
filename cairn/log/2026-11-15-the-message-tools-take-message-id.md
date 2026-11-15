---
cairn: log
change: the-message-tools-take-message-id
landed: 2026-11-15
---

# The message tools take `message-id`

Task H5 of the family's cli-alignment-2026-11-15 plan (finding F17).

## What landed

- `message read`, `message reply` and `message forward` name their positional `MESSAGE-ID`, so their MCP tools take `message-id`, as `attachment_list` and `attachment_download` do; `id` is refused as an unknown argument, coded `usage-invalid`. The server's instructions say the envelope id is passed as `message-id`.
- tests/mcp.rs `a_message_is_named_message_id`; the composer tests call with `message-id`.
- CHANGELOG (**BREAKING**) and MIGRATION.md, under "From a development build of v3".

## Capabilities moved

- [mcp](../spec/mcp.md): *The server tells the agent how to use it*, *A tool's input is the command's grammar* and its scenario.
