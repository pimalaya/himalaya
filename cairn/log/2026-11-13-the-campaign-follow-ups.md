---
cairn: log
change: the-campaign-follow-ups
landed: 2026-11-13
---

# The campaign follow-ups

Three questions the 2026-11-12 live campaign left open, settled by the owner on 2026-11-13. A send on Microsoft Graph ignores `--save`, `--no-save` and `message.send.save-copy`, Graph filing every sent message in Sent Items itself, and its receipt says `saved: true`; the save of a send is checked against `submit.copy`, the sending transport's, so an account setting `message.send.save-copy` no longer fails every Graph send. `vacation set` clears a field with `--no-from`, `--no-to`, `--no-subject`, `--no-text` or `--no-html`, on io-jmap's `JmapVacationResponseUpdate` fields made `Option<Option<String>>` (breaking there, logged as vacation-fields-clearable) and on Gmail by unsetting the field before its full update. A unified leaf lists the capabilities only an argument reaches (`Leaf::optional`): the `--help` footer names them with their arguments, `message compose` gets its footer, and an MCP tool leaves out an argument the account cannot serve rather than the whole tool. Capabilities moved: commands ("Saving a sent message follows the send", the vacation and footer requirements), mcp ("Tools are the classified unified API").
