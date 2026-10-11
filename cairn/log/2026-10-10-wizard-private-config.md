---
cairn: log
change: wizard-private-config
landed: 2026-10-10
---

# Save generated accounts privately

New Unix configuration files are created exclusively with mode 0600, and new parent directories use mode 0700. Appending restricts the opened file to mode 0600 before writing, retaining the original bytes. Existing parent directories are not chmodded.

Updated wizard with the private configuration requirement. Tests cover nested directory and file modes, restricting an existing append target, preserving comments, refusing to truncate an existing file and refusing a dangling symlink on creation.
