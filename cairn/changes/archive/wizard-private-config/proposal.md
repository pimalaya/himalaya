---
cairn: change
id: wizard-private-config
status: landed
created: 2026-10-10
---

# Save generated accounts privately

## Why
The wizard can render raw credentials but creates configuration files using only the process umask, leaving them readable by other users under a common umask.

## What
Create new configuration files exclusively with Unix mode 0600 and missing parent directories with mode 0700. Keep the existing plain text append and its permissions. Use only the standard library, as requested in the maintainer's review of PR #784.
