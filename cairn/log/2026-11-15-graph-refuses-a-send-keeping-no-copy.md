---
cairn: log
change: graph-refuses-a-send-keeping-no-copy
landed: 2026-11-15
---

# Graph refuses a send keeping no copy

Task H18 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15: `message send --no-save` on Graph printed "saved and sent", Graph filing the copy whatever is asked.

## What landed

- src/capability.rs: `FILES_SENT_COPY` (Graph) and `Resolution::require_no_copy`, refusing `--no-save` coded `capability-missing`: ``Account `a` cannot send keeping no copy (`--no-save`) over msgraph: it files every message it sends in its sent mailbox itself``. The Graph `submit.copy` detail reads that `--save` and `message.send.save-copy` are ignored, `--no-save` refused.
- src/cli.rs: `run_shared` takes `no_copy`, checked with the capabilities before any connection; `MessageCommand::keeps_no_copy` gives it for `message send`, `compose`, `reply` and `forward`.
- tests/capability.rs `graph_refuses_a_send_keeping_no_copy`: refused before the token command (`false`) runs, which the old code ran, failing uncoded; Gmail goes on to connect.
- Refusing was chosen over rewording the receipt: the resolution knows the sending transport without connecting, and a refusal tells the caller its wish cannot be met, where a truthful "saved" would still drop the flag silently.

## Capabilities moved

- [commands](../spec/commands.md): *Saving a sent message follows the send* refuses `--no-save` on Graph and gains the scenario *A Graph send keeping no copy*.
