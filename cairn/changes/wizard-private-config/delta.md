---
cairn: delta
change: wizard-private-config
---

## ADDED Requirements

### Requirement: Generated configuration files are private
On Unix, the wizard SHALL create new configuration directories with mode 0700 and new configuration files with mode 0600, subject to a more restrictive umask. Before appending an account, it SHALL restrict the opened file to mode 0600. Creating a new configuration SHALL fail rather than truncate an existing entry.
