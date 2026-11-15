---
cairn: delta
change: the-message-tools-take-message-id
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: The server tells the agent how to use it
`initialize` SHALL carry `instructions` saying how to find a mailbox (`mailbox_list`, the `mailbox` argument, the inbox when left out) and an envelope id (`envelope_list`, `envelope_search` and its query language), passed as `message-id`, that `attachment_download` returns the bytes as an embedded resource and the composers take files inline as `attach-data`, that `--allow-write` and `--allow-send` gate the tools and arguments that change the store and send mail, that `account_capability` says which backend serves each capability of the account and how well, and that a failed call answers `isError` with the `himalaya-error` object. Every tool name it holds SHALL be one of the tool table.

### Requirement: A tool's input is the command's grammar
A tool's `inputSchema` SHALL be generated from its clap command by pimalaya-cli's `mcp::grammar::Grammar`, each property named as the CLI spells the argument, with `additionalProperties: false`, and a call SHALL be parsed back by that same command from the argv the grammar writes. The one exception SHALL be the server-only arguments its tool declares, which clap never sees. An argument the `inputSchema` does not declare SHALL be refused before the command runs as a tool error ``Unknown argument `<key>`, <tool> takes <properties>``, the server-only ones named among them. A value of the wrong type, and a count other than one for an argument the tool takes one value of, SHALL be refused the same way. Each of these refusals, and a value the command's grammar refuses once the argv is parsed (a negative `page-size`), SHALL carry the code `usage-invalid`, the last with clap's message without its `error: ` prefix. A tool's property takes a lone value where the command repeats one, so `query` is a string or an array of words; `page` and `page-size` are integers; `attachment_download` takes `attachment-id` as one string. A tool naming one message SHALL take it as `message-id`, as the CLI names it: `message_read`, `message_reply`, `message_forward`, `attachment_list` and `attachment_download`.

#### Scenario: A message named by its id
- GIVEN a server over a Maildir holding the message `m1`
- WHEN `message_read` is called with `{"message-id": "m1"}`, then with `{"id": "m1"}`
- THEN the first answers the read, `id` `m1` and `etag` `null`, and the second is refused coded `usage-invalid`, naming `message-id` among the properties the tool takes

## REMOVED Requirements

None.
