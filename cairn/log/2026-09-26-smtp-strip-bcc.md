---
cairn: log
change: smtp-strip-bcc
landed: 2026-09-26
---

# Blind recipients stay blind over SMTP

Reported in #747, first fixed at the app level in #761 by @gianlucamazza. The SMTP adapter derived the envelope from `To:`, `Cc:` and `Bcc:`, then handed the raw message to `DATA` unchanged, so every recipient could read the blind-copied addresses.

The removal landed in io-smtp 0.4 instead, so every Pimalaya client gets it: `SmtpMessageSend` removes `Bcc` from the transmitted header section unless `SmtpMessageSendOptions::keep_bcc` is set. Himalaya bumps io-smtp and passes the default from `src/smtp/backend.rs`.

`smtp send` passes `keep_bcc: true`. It is the protocol-level command, its envelope is given explicitly, and its contract is to transmit the message as given. The `--save` copy is untouched too, since the sender needs to see who was blind-copied.

No `--keep-bcc` flag or configuration option was added: on the shared commands it could only leak the field, and a caller who really wants the bytes verbatim has `smtp send`.

Spec updated: `backends` (MODIFIED: "Sending transport").
