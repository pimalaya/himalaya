---
cairn: change
change: plain-output-is-printable
---

# Delta

## ADDED Requirements

### Requirement: Plain output is printable
A single-line field taken from a message or a server (a subject, a name, a filename, a mailbox name, an id) SHALL have its control characters (C0, DEL and C1) and bidi controls (U+202A to U+202E, U+2066 to U+2069) replaced with U+FFFD before it is printed as plain output, so it can neither drive the terminal nor reorder what is displayed. Message bodies keep their tabs and newlines; on a terminal they SHALL be refused as binary when they hold any other control character, DEL and C1 included. The `--json` output SHALL keep the strings unchanged.
