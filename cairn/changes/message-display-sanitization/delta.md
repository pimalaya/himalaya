---
cairn: delta
change: message-display-sanitization
---

## MODIFIED Requirements

### Requirement: Plain output is printable
Single-line fields from messages and servers SHALL replace control and bidi characters with U+FFFD. The designed message view SHALL replace those characters in decoded bodies too, preserving tabs and line feeds, on terminals and redirected stdout alike. JSON strings and explicitly requested raw bytes SHALL remain unchanged.
