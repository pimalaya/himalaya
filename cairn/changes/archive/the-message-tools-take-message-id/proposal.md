---
cairn: change
id: the-message-tools-take-message-id
status: landed
created: 2026-11-15
---

# The message tools take `message-id`

## Why

Task H5 of the family's cli-alignment-2026-11-15 plan (finding F17). The MCP tools `message_read`, `message_reply` and `message_forward` named the message `id`, the lowercased value name `ID` of their positional, while `attachment_list` and `attachment_download` named it `message-id`, and Cardamum and Calendula name theirs `card-id`, `event-id`, `item-id`. One tool server, two names for one thing.

## What

The positional of `message read`, `message reply` and `message forward` is named `MESSAGE-ID`, so the tool property pimalaya-cli's grammar derives from it is `message-id`; the CLI is unchanged but for the `<MESSAGE-ID>` its help prints. The server's instructions say the envelope id is passed as `message-id`. Breaking for MCP clients of a development build of v3, `id` being refused as an unknown argument: CHANGELOG and MIGRATION.md say so.
