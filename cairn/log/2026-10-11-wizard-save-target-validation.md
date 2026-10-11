---
cairn: log
change: wizard-save-target-validation
landed: 2026-10-11
---

# Validate named wizard save targets

The named configuration wizard reads a regular descriptor nonblocking on Unix, retains its handle through prompts to prevent inode reuse, and compares the reopened target's identity and contents before appending. Special files, changed contents and replaced inodes are refused before permission changes or generated-byte writes. Unchanged symlink configurations remain supported.

macOS write targets and newly created directories with ACL entries are refused, without silently deleting user ACLs. ACL inspection errors fail closed except Darwin's documented absence cases. A failed creation may leave empty entries, but no generated credentials. The containing directories remain trusted, concurrent hostile same-user writes and Windows ACL hardening are not covered, and other commands' general configuration loader is unchanged.

Added seven regression tests covering explicit/inherited ACLs, FIFO targets with and without a reader and descriptor validation, content/inode replacement, repeated unlink/recreation while holding the original descriptor, and symlink compatibility. Sixteen wizard tests and the all-features suite (247 unit successes, six integration successes, one existing ignore) passed on macOS arm64. Clippy with warnings denied, formatting, cargo deny, reduced IMAP/SMTP and JMAP builds, a wizard/Maildir-only test build and the 64-test ManageSieve-only suite passed; reduced builds retain existing warnings.

OpenAI Codex implemented and tested the patch. An independent gpt-6-astra subagent identified the inode-reuse gap in the first iteration and found no remaining blocking issue after the descriptor-retention correction. It inspected source and did not independently rerun the test suites.
