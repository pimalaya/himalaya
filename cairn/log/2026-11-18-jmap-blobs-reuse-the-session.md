---
cairn: log
change: jmap-blobs-reuse-the-session
landed: 2026-11-18
---

# JMAP blobs reuse the session

Finding HIM-13 of the family's cli-family-analysis-2026-11-15 plan, its JMAP half.

## What landed

- src/jmap/client.rs: the resolved credential kept, `auxiliary` reusing one connection per foreign host; src/jmap/email/import.rs through it.
- Test `blob_downloads_reuse_the_credential_and_the_connection`, failing on e440b4e4.
- CHANGELOG (Fixed). Left: the Graph folder-tree walk of every one-shot command.

## Capabilities moved

- [provider-quirks](../spec/provider-quirks.md): the JMAP download host paragraph.
