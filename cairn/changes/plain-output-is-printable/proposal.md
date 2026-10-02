---
cairn: change
id: plain-output-is-printable
status: active
created: 2026-10-02
---

# Keep control characters out of the plain output

## Why

The tables print subjects, display names, filenames and other strings exactly as the message or the server supplied them, so anyone who can send a message can put terminal escape sequences in front of the reader. A subject ending in `ESC ] 52 ; c ; … BEL` rewrites the clipboard in terminals that honour OSC 52, `ESC [ 8 m` hides the rest of the line and `ESC [ 2 J` clears the screen. The `--json` output escapes the C0 characters, ESC included, and is left as it is.

## What

`printable` in src/shared/table.rs replaces every control character (C0, DEL and C1) with U+FFFD, and borrows the string when there is none. It is applied at the cell, where a string from a message or a server is rendered, so the data and the `--json` output keep the original string.

This change starts with the listings whose content any sender controls, `envelope list` and `envelope search` (one renderer) and `attachment list`. The other plain renderings of server-supplied strings are listed in the tasks and follow once the approach is agreed.
