---
cairn: change
id: message-read-invalid-recipients
status: landed
created: 2026-10-06
---

# The read view lists the address entries it cannot read

## Why

The `message read` view drops an address entry it cannot read as an address (a bare word, `a@localhost`, a quoted local part, a name with no address), so a client can no longer see it. A client that shows a draft for confirmation before it is sent, as MOA's send preview does, must refuse such a draft rather than let the user confirm recipients that differ from what will be sent.

## What

- `headers.invalid`: an object `{to, cc, bcc, replyTo, from, sender}`, each a list of the entries of that header the address parser dropped, as decoded text (encoded words decoded, trimmed), in header order, empty when none.
- A group name (`Team:` … `;`) is not an entry; an empty group (`undisclosed-recipients:;`) yields nothing.
- The valid lists are unchanged. `message read --json` and `message parse --json` print the same, through the same code.
