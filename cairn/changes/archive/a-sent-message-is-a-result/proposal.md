---
cairn: change
id: a-sent-message-is-a-result
status: landed
created: 2026-11-18
---

# A sent message is a result

## Why

Finding HIM-11 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18 (ruling D26, with COH-5): a send that went out but whose follow-up failed printed the failure object and exited 1, the delivery told by free wording alone, so a script retrying sent the message twice. `message reply --send` whose answered flag failed printed the receipt and then a failure object, two values on stdout.

## What

- A copy failing after the send (`--save`, `message.send.save-copy`, `message add --send`, Gmail filing the sent message) is an item error of the receipt, `sent: true`, `saved: false`, exit 3.
- A reply's answered flag failing after the send is an item error of the one receipt, its `id` the source message, exit 3.
- `MessageRouteOutput` (`message send`, the composers) and `MessageAddOutput` gain `errors`, `[]` when nothing failed.
- Breaking: exit 1 becomes 3 for these outcomes, the failure object becomes the receipt, and two receipts gain `errors`.
