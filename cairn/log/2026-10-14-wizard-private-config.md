---
cairn: log
change: wizard-private-config
landed: 2026-10-14
---

# Save generated accounts privately

New configuration files use exclusive creation and Unix mode 0600. Missing parent directories use mode 0700. Existing files retain their permissions and bytes when a generated account is appended, and existing directories retain their permissions. The implementation uses the standard library without snapshots, FIFO handling or ACL FFI, following the maintainer's PR #784 review.

Folded the private creation requirement into wizard and updated the changelog. Regression tests cover nested creation modes, existing permissions and contents, occupied files, dangling symlinks and missing append targets. The all-feature suite passed 273 tests with one ignored. Formatting, diff checks and all-target/all-feature clippy with warnings denied passed. Clippy used the system OpenSSL because vendored assembly generation failed in this environment.
