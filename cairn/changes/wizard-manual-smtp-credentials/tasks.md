---
cairn: tasks
change: wizard-manual-smtp-credentials
---

- [x] Ask "Use the same credentials for SMTP?" in the manual flow, after the IMAP config is built
- [x] On accept, reuse the IMAP SASL and prompt only the SMTP endpoint (host seeded from the IMAP host, encryption, port)
- [x] On decline, keep the full SMTP prompts
- [x] Update the wizard spec requirement and CHANGELOG
