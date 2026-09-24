---
cairn: change
id: smtp-strip-bcc
status: landed
created: 2026-09-24
---

# Keep blind carbon copies blind over SMTP

## Why

The SMTP adapter builds the RFC 5321 envelope from the message's `To:`, `Cc:` and `Bcc:` fields, which is right, and then transmits the raw message unchanged, which is not. The `Bcc:` field reaches every recipient, so the addresses it was meant to hide are disclosed to all of them. RFC 5322 section 3.6.3 describes the field as one removed before the message is sent to the other recipients.

It affects every account that sends through SMTP (IMAP, Maildir and m2dir storage), whichever command produced the message: `message send`, and `compose`, `reply` and `forward` with `--send`.

## What

`send_message` still derives the envelope from all three fields, then removes every `Bcc:` field from the header section before `DATA`, continuation lines included. The name is matched case-insensitively and with the obsolete whitespace before the colon. The body is left alone, so a line reading `Bcc:` after the blank line survives.

## What this is not

The saved copy is not changed. `--save` stores the message as composed, with its `Bcc:`, which is where the sender needs to see who was blind-copied. The self-sending backends are not touched either, because their APIs take the recipients and handle the field themselves.
