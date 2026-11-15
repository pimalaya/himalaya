---
cairn: change
id: graph-refuses-a-send-keeping-no-copy
status: landed
created: 2026-11-15
---

# Graph refuses a send keeping no copy

## Why

Task H18 of the family's cli-alignment-2026-11-15 plan, from live testing on 2026-11-15: on Graph, `message send --no-save` printed "Message successfully saved and sent" (`saved: true`), Graph's raw MIME send filing every message in Sent Items whatever is asked. The receipt was true and the flag silently ignored.

## What

`--no-save` on a send whose transport files every sent message itself (Graph) is refused up front, coded `capability-missing` as the family refuses an option a backend does not serve, before the client is built, so no token is asked for. Refusing is possible, the resolution knowing the sending transport without connecting, so the text is not reworded instead: a caller asking for no copy learns it cannot have one. `--save` and `message.send.save-copy` stay ignored on a send there, a copy being kept as asked, in Sent Items. No capability is invented for it: `submit.copy` stays `partial`, its detail now saying `--no-save` is refused, and an MCP tool keeps its `no-save` argument, answering the refusal.
