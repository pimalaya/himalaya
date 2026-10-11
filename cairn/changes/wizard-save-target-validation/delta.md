---
cairn: delta
change: wizard-save-target-validation
---

## MODIFIED Requirements

### Requirement: Generated configuration files are private
On Unix, generated files use mode 0600 and new directories use mode 0700. On macOS, ACL-bearing write targets and newly created directories are refused before writing secrets. Existing parent directories remain unchanged and must be trusted. This does not claim Windows ACL hardening or protection from hostile same-user concurrent writes. Failed creation may leave empty filesystem entries but no generated credentials.

## ADDED Requirements

### Requirement: The named wizard validates its save target
The named wizard reads regular descriptors through nonblocking Unix opens and retains the original handle while prompting. Append verifies regular-file type, original contents, and Unix device/inode identity before chmod or generated-byte writes. Changed or replaced configurations are refused. Unchanged symlink configurations remain supported. The general account loader and bare invocation are outside this change.
