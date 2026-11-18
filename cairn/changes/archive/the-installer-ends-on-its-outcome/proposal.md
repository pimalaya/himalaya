---
cairn: change
id: the-installer-ends-on-its-outcome
status: landed
created: 2026-11-18
---

# The installer ends on its outcome

## Why

Finding PKG-1 of the family's cli-family-analysis-2026-11-15 plan: install.sh:65 read `die"…"` with no space after `die` since 2026-11-11 (aa425b48), so a successful install printed `command not found` and exited 127, and `curl ... | sh && himalaya ...` stopped. The owner's ruling of 2026-11-18 adds the download flags of carillon d575ac2: `curl -sLo` saved an HTTP error page as the archive.

## What

- The space after `die`.
- `curl -fsSLo`, failing on an HTTP error.
