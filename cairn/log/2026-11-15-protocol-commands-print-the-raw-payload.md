---
cairn: log
change: protocol-commands-print-the-raw-payload
landed: 2026-11-15
---

# Protocol commands print the raw payload

Lines X11 and D26 of the family's cli-alignment-2026-11-15 plan, round X13.

## What landed

- JMAP: `jmap thread get` and `jmap identity get` send their `Foo/get` through src/jmap/raw.rs (an identity under the submission capability, every one when given no id) and print each object as sent beside `state`; `jmap submission create` sends `EmailSubmission/set` raw and prints the created submission as the server answered it, beside `oldState` and `newState`.
- Gmail: `GmailClient::raw_list_at` and `raw_get`; `gmail labels list` and the four settings lists print Gmail's answer under its key; `gmail labels get` takes one id and prints the label.
- Graph: `MsgraphClient::raw_get`; `msgraph mail-folders get` takes one id and prints the folder; `msgraph profile get` prints Graph's user.
- `gmail request` and `msgraph request` print `{response}` (src/shared/raw_json.rs); `jmap request` takes one request object and prints `{response}`, the method-calls form kept as the hidden `jmap query`.

## Capabilities moved

- [commands](../spec/commands.md): *The Gmail and Graph escape hatches are requests* added; *The JMAP escape hatch is a request*, *Protocol listings print the provider's payload*, *An option after the words is an option* and the text forms of `gmail labels get` and `msgraph mail-folders get` modified.
