---
cairn: change
id: message-display-sanitization
status: landed
created: 2026-10-10
---

# Sanitize the human-readable message view

## Why
Decoded bodies and MIME headers reach stdout verbatim, allowing mail to send terminal commands through the normal message view.

## What
Sanitize single-line fields with the existing shared sanitizer. Replace control and bidi characters in decoded bodies while retaining tabs and newlines. Preserve JSON strings and explicitly requested raw bytes. The user approved these security fixes and fork-first validation before implementation on 2026-10-10.
