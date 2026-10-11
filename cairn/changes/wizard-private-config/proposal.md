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
On Unix, create new configuration directories with mode 0700 and configuration files with mode 0600. Before appending, restrict the opened file to owner read/write. Create a new configuration exclusively so concurrent changes cannot be truncated. Keep existing parent directory permissions and file content intact. The user approved these security fixes and fork-first validation before implementation on 2026-10-10.
