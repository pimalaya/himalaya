---
cairn: change
id: the-mcp-mbox-stays-in-its-root
status: landed
created: 2026-11-18
---

# The MCP mbox stays in its root

## Why

Finding HIM-1 of the family's cli-family-analysis-2026-11-15 plan, rechecked on the live head 2026-11-18. Over an mbox account the MCP server read and appended to an mbox outside `mbox.root`: `MboxClient::resolve_mbox` opens an absolute path given as a mailbox, a rule written for the command line (as `mail -f` does), and the server's path closing was consulted only by the message argument. `envelope_list {"mailbox": "/abs/other.mbox"}` listed another archive, `message_read` read it, `message_add` under `--allow-write` appended to it, and a mail read through the tools can steer the model there. cairn/spec/mcp.md says no tool takes a local path. A relative name holding `..` was already refused by io-mbox.

## What

- `resolve_mbox` (src/mbox/client.rs) refuses an absolute mailbox when `mcp::paths_closed()`, every mbox operation (reads, writes, rename, delete) going through it.
- The command line keeps opening any mbox by its absolute path (io-family-audit MBOX-08, by design).
