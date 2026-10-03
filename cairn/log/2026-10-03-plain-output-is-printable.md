---
cairn: log
change: plain-output-is-printable
landed: 2026-10-03
---

# Plain output keeps control and bidi characters out of the terminal

Subjects, names, filenames, mailbox names and ids reached the plain output as the message or the server supplied them, so a sender could drive the terminal with escape sequences (rewriting the clipboard with OSC 52, hiding text, clearing the screen) or disguise a filename with bidi overrides.

The single-line fields of the envelope, attachment, mailbox, ID, pimdir queue, JMAP, Gmail and Graph outputs now go through `pimalaya_cli::table::sanitize` from pimalaya-cli 0.2.6, which replaces C0, DEL, C1 and the bidi controls with U+FFFD. JMAP set errors are sanitized once, in `format_set_error`. Message bodies keep their tabs and newlines; `write_bytes_or_save` now refuses DEL and C1 on a terminal as well, reading bytes that are not UTF-8 as Latin-1. The `--json` output is unchanged.

Capabilities moved: **commands** (plain output is printable).

Verified with unit tests rendering the envelope and attachment tables from strings holding OSC 52, SGR and screen-clearing sequences, and with tests of the binary check for C0, DEL, UTF-8 encoded and raw C1, and UTF-8 text whose continuation bytes fall in 0x80..=0x9f.
