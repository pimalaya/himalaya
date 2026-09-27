---
cairn: log
change: gmail-draft-write-identities
landed: 2026-09-19
---

# Gmail draft write identities

Gmail draft create and update now return structured identities under `--json` instead of a confirmation wrapper, fixing #756. The shared output type retains the text confirmation and registers both command schemas. No request or draft lifecycle behavior changes.

Spec updated: commands (ADDED: Gmail draft writes return identities).
