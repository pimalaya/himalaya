---
cairn: delta
change: wizard-manual-smtp-credentials
---

## MODIFIED Requirements

### Requirement: Per-protocol test and shared SMTP credentials
The discovered IMAP + SMTP flow SHALL test each protocol as it configures it: the IMAP connection is validated first, then the wizard asks whether SMTP reuses the same credentials (the two may advertise different auth), re-running the SASL prompt for a distinct one, and tests SMTP last. The manual IMAP + SMTP flow SHALL likewise ask whether to reuse the IMAP credentials for SMTP: when accepted it reuses the IMAP SASL and prompts only the SMTP endpoint (host seeded from the IMAP host, encryption, port); when declined it runs the full SMTP prompts. JMAP and the proprietary APIs are validated by the account test. A backend that validates itself inline skips the final account test.
