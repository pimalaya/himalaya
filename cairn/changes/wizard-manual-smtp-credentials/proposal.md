---
cairn: change
id: wizard-manual-smtp-credentials
status: landed
created: 2026-07-28
---

# Manual wizard offers to reuse IMAP credentials for SMTP

## Why
The discovered IMAP + SMTP flow asks whether SMTP reuses the IMAP credentials, so they are entered once. The manual flow did not: it always ran the full SMTP prompts, forcing the user to re-type the same login and secret they had just given for IMAP.

## What
Mirror the discovered flow's credential reuse in the manual flow. After the IMAP config is built, the wizard asks whether to use the same credentials for SMTP. When accepted, it reuses the IMAP SASL config and prompts only the SMTP endpoint (host, encryption, port), seeding the host from the IMAP host since a provider often shares one hostname. When declined, the full per-protocol SMTP prompts run as before. The SMTP endpoint is still prompted rather than derived, because nothing was discovered in the manual flow.
