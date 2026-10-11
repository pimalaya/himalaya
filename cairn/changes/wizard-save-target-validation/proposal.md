---
cairn: change
id: wizard-save-target-validation
status: landed
created: 2026-10-11
---

# Validate wizard save targets

The user requested implementation of the independent review findings on PR #784. Mode bits alone do not constrain macOS ACL grants, and a special file substituted during prompts can block or receive the generated document.

Reject macOS ACL-bearing targets rather than silently deleting user ACLs. Open existing files nonblocking on Unix, validate regular file descriptors, and compare the original identity and contents before appending. The containing directories remain trusted. Update PR #784 without replacing its history.
