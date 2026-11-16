---
cairn: change
id: the-local-stores-add-a-message
status: landed
created: 2026-11-16
---

# The local stores add a message

## Why

Task L18 of the family's cli-alignment-2026-11-15 plan, owner ruling D32 (2026-11-16). pimalaya-cli cairn/spec/commands.md, *A verb keeps one meaning*: `add` SHALL create an item and `create` a collection, a binary spelling one otherwise keeping the old verb as a hidden alias. Himalaya's local namespaces delivered a message with `maildir message save` and `mbox message save`, where the unified API and `pimdir message add` say `add`. The protocol namespaces keep the protocol's verb (`imap append`, the JMAP, Graph and Gmail creates) and are not touched.

## What

- `maildir message save` becomes `maildir message add`, `mbox message save` becomes `mbox message add`; `save` stays a hidden alias of each, parsing to the same command.
- The types and files follow: `MaildirMessageAddCommand` and `MaildirMessageAddOutput` in src/maildir/message/add.rs, `MboxMessageAddCommand` and `MboxMessageAddOutput` in src/mbox/message/add.rs. The text confirmation reads `successfully added`.
- The JSON schema keys follow the command path: `himalaya-maildir-message-add` and `himalaya-mbox-message-add`, where v2.2.1 shipped `himalaya-maildir-message-save` and `himalaya-mbox-message-save`. Breaking for a script reading those schemas by name: CHANGELOG **BREAKING** line and a MIGRATION.md entry.
- MCP is untouched: `himalaya mcp` exposes the unified API alone (src/mcp.rs `TOOLS`), no raw `maildir` or `mbox` command, so no tool name changes and no tool reaches the hidden alias.
- Completions and manual pages are generated from clap and follow; a hidden alias is in neither.
