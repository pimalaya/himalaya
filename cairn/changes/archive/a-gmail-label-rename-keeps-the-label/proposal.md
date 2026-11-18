---
cairn: change
id: a-gmail-label-rename-keeps-the-label
status: landed
created: 2026-11-18
---

# A Gmail label rename keeps the label

## Why

Finding HIM-20 of the family's cli-family-analysis-2026-11-15 plan: `gmail labels update` renamed through `labels.update`, a PUT carrying only the id and the name, so the label lost its colour and visibility. The unified `mailbox rename` moved to `labels.patch` for that reason; the namespace command kept the PUT.

## What

`gmail labels update` sends `labels.patch`.
