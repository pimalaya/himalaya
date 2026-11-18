---
cairn: log
change: the-mcp-mbox-stays-in-its-root
landed: 2026-11-18
---

# The MCP mbox stays in its root

Finding HIM-1 of the family's cli-family-analysis-2026-11-15 plan.

## What landed

- src/mbox/client.rs: `resolve_mbox` refuses an absolute path inside an MCP server.
- Test `an_mbox_outside_the_root_is_refused_by_the_server` (tests/mcp.rs), failing before the fix.
- CHANGELOG (Fixed).

## Capabilities moved

- [backends](../spec/backends.md): *mbox backend*.
- [mcp](../spec/mcp.md): *A tool reads and writes no local file*.
