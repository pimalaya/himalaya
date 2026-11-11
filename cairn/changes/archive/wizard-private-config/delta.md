---
cairn: delta
change: wizard-private-config
---

## ADDED Requirements

### Requirement: Generated configuration files are private
The wizard SHALL create new configuration files exclusively, failing rather than overwriting an existing entry. On Unix, it SHALL create new files with mode 0600 and missing parent directories with mode 0700, subject to a more restrictive umask. It SHALL NOT change the permissions of existing files or directories. Appending SHALL preserve the existing bytes and add the generated account as plain text.
