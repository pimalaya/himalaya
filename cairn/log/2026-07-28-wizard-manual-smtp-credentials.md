---
cairn: log
change: wizard-manual-smtp-credentials
landed: 2026-07-28
---

# Manual wizard offers to reuse IMAP credentials for SMTP

Brought the discovered flow's credential reuse to the manual flow. After building the IMAP config, the wizard now asks whether to use the same credentials for SMTP. On accept it reuses the IMAP SASL and prompts only the SMTP endpoint (host, encryption, port), seeding the host from the IMAP host since a provider often shares one hostname; on decline it runs the full SMTP prompts as before. This moved the wizard capability forward by extending the shared-SMTP-credentials requirement to cover the manual flow.
