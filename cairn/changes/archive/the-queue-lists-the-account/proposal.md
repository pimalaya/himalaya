---
cairn: change
id: the-queue-lists-the-account
status: landed
created: 2026-11-18
---

# The queue lists the account

## Why

Finding TRI-5 of the family's cli-family-analysis-2026-11-15 plan, ruled by the owner on 2026-11-18: `himalaya pimdir queue list` covered one mailbox, the inbox when `-m` was omitted, where the twins list the whole account. A queued send is anchored on the sent mailbox and logs "see `himalaya pimdir queue list`", which then showed the inbox: empty. The standing refusals show under `refused` (ledger line X18 of the family's cli-alignment-2026-11-15 plan).

## What

- `pimdir queue list` lists the whole account, mailbox by mailbox in store order; `-m/--mailbox` narrows it to one mailbox, no longer defaulting to the inbox.
- `refused` lists the refusals standing in what is listed, as before.
- The text form names the mailbox only when one is narrowed to.
- Breaking: a bare `pimdir queue list` now lists every mailbox, not the inbox.
