---
cairn: log
change: attachment-exclusive-create
landed: 2026-10-10
---

# Create attachment files exclusively

Replaced filename existence checks followed by truncating writes with `create_new` reservations. Occupied files, directories and leaf symlinks are skipped, and exhausting 1024 candidates fails without replacing any entry. Concurrent downloads reserve distinct files.

Updated commands with the attachment non-replacement requirement. Regression tests cover normal collisions, dangling symlinks, exhaustion and eight concurrent writers.
