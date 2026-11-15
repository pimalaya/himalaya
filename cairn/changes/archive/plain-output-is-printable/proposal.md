---
cairn: change
id: plain-output-is-printable
status: landed
created: 2026-10-02
---

# Keep control characters out of the plain output

## Why

The tables print subjects, display names, filenames and other strings exactly as the message or the server supplied them, so anyone who can send a message can put terminal escape sequences in front of the reader. A subject ending in `ESC ] 52 ; c ; … BEL` rewrites the clipboard in terminals that honour OSC 52, `ESC [ 8 m` hides the rest of the line and `ESC [ 2 J` clears the screen. The `--json` output escapes the C0 characters, ESC included, and is left as it is.

## What

`pimalaya_cli::table::sanitize` (pimalaya-cli 0.2.6, shared with cardamum and calendula) replaces every control character (C0, DEL and C1) and the bidi controls with U+FFFD, and borrows the string when there is none. It is applied where a single-line field from a message or a server is rendered: the envelope, attachment, mailbox, ID, pimdir queue, JMAP, Gmail and Graph outputs. Message bodies keep their tabs and newlines; the binary check of `write_bytes_or_save` now also refuses DEL and C1 on a terminal. The data and the `--json` output keep the original strings.
