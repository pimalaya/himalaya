---
cairn: delta
change: message-send-save-copy
---

## ADDED Requirements

### Requirement: Sent copies are configured per account
`message.send.save-copy`, global or per account, SHALL be the mailbox a sending command (`message send`, and `message compose`, `message reply` and `message forward` with `--send`) appends a copy to when `--save` is not passed. It takes a mailbox name, alias or role, resolved as `--save` is; `true` SHALL stand for `sent` and `false` for no copy. `--no-save` SHALL skip it for one call and conflict with `--save`. The `message` table SHALL accept unknown keys, so a v1 `[message]` table keeps loading.

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL fail the command with an error stating that the message was sent.
