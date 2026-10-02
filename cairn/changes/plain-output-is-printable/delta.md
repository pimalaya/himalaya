---
cairn: change
change: plain-output-is-printable
---

# Delta

## ADDED Requirements

### Requirement: Plain output is printable
A string taken from a message or a server SHALL have its control characters (C0, DEL and C1) replaced with U+FFFD before it is printed as plain output, so it cannot drive the terminal. The `--json` output SHALL keep the string unchanged.
