---
cairn: log
date: 2026-09-24
change: smtp-strip-bcc
---

# Blind carbon copies stay blind over SMTP

The SMTP adapter derived the envelope from `To:`, `Cc:` and `Bcc:` and then handed the raw message to `DATA` unchanged, so the `Bcc:` field reached every recipient and disclosed the addresses it exists to hide.

## What landed

**`strip_bcc` runs on the bytes handed to `DATA`,** after the envelope is derived. It walks the header section only, drops every field named `Bcc` (case-insensitive, with the obsolete whitespace before the colon) together with its folded continuation lines, and copies the body verbatim from the first empty line.

**The saved copy is unchanged:** `--save` appends the message as composed, where the sender needs to see the blind recipients. The self-sending backends are untouched.

**Checked over SMTP (OVH):** a message To one address and Bcc another reached both. The copy delivered to the blind recipient carried no `Bcc:` field, and the saved copy in Sent kept it.

## Capabilities moved

- [backends](../spec/backends.md): *Sending transport* now requires the `Bcc:` field removed from the transmitted message.
