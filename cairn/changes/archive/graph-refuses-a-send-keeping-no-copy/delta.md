---
cairn: delta
change: graph-refuses-a-send-keeping-no-copy
---

# Delta

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Saving a sent message follows the send
A command both saving and sending (`--save` with `--send`, `message send --save`, `message add --send`) SHALL send first and save afterwards, so a failed send leaves no copy. A save failing after a successful send SHALL fail the command with an error stating that the message was sent. A backend filing every sent message itself (Gmail, under `SENT`) SHALL file the saved copy by labelling the message it sent with the mailbox and the flags, rather than add a second message, and SHALL report that message's id; pimdir files it through its queue. The save of a send SHALL be checked against `submit.copy`, the sending transport's, and a save alone against `message.add`. Graph files every message it sends in Sent Items itself and declares `submit.copy` `partial` with that detail: on a send, `--save` and `message.send.save-copy` SHALL be ignored, nothing added, and the receipt SHALL say `saved: true` with a `null` id. `--no-save` SHALL be refused there before connecting, coded `capability-missing` and naming the flag, Graph keeping the copy whatever is asked; a save without a send stays refused, `message.add` being `none` (backends: "Append gap").

#### Scenario: A Graph send keeping no copy
- GIVEN an account whose storage backend is Graph
- WHEN `himalaya --json message send --no-save <message>` runs, or `message compose --send --no-save`
- THEN it fails coded `capability-missing`, naming `--no-save`, before the token is asked for

## REMOVED Requirements

None.
