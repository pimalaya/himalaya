---
cairn: delta
change: smtp-strip-bcc
---

# Delta

## MODIFIED Requirements

### Requirement: Sending transport
Backends that self-send (JMAP, Gmail, Graph) SHALL route `send_message` through their own API. Storage backends that cannot send (IMAP, Maildir, m2dir) SHALL send through the account's SMTP transport, adapted in `src/smtp/backend.rs` over io-smtp, which parses the RFC 5321 envelope from the raw message headers. The transmitted message SHALL NOT carry the `Bcc:` field (RFC 5322 section 3.6.3): it is removed from the header section, continuation lines included, after the envelope is derived from it.
