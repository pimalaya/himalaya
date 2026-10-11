---
cairn: log
change: message-display-sanitization
landed: 2026-10-10
---

# Sanitize the human-readable message view

Applied the existing single-line sanitizer to address fields, subjects, attachment names and MIME metadata. Decoded text and HTML-only bodies replace control and bidi characters with U+FFFD, retaining tabs and line feeds and normalizing CRLF to LF. JSON strings and explicit raw bytes are preserved.

Updated commands and the command help. Regression tests cover OSC 52, screen controls, C1, DEL, bare CR, bidi overrides, MIME headers, HTML-only messages and JSON preservation.
