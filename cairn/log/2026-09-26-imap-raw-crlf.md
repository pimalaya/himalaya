---
cairn: log
change: imap-raw-crlf
landed: 2026-09-26
---

# IMAP raw terminates the last command with CRLF

Reported in #764: `himalaya imap raw -- 'a1 NOOP'` hung until the stream timed out against Gmail. When the last command carried no terminator, `imap raw` appended a bare LF. io-imap accepted it as a complete line, but Gmail never answers a bare-LF command, so the tagged completion never came. Commands piped through stdin always hit it, since their lines are joined with CRLF but the last one keeps none.

The appended terminator is now a CRLF, as the spec already required.

Spec unchanged.
