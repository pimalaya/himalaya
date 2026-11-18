---
cairn: change
id: downloaded-attachments-are-private
status: landed
created: 2026-11-18
---

# Downloaded attachments are private

## Why

Finding HIM-10 of the family's cli-family-analysis-2026-11-15 plan, the half the first batch left: with no `downloads-dir` and no XDG download directory, `attachment download` wrote into the shared temporary directory at 0644 under umask 022.

## What

Every attachment file is created 0600, through a local `private_options`, the minimal form of pimalaya-cli's `file::private_options`; Himalaya does not depend on that helper while pimalaya-cli has not released it, and moves to it once it does.
