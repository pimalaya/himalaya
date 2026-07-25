---
cairn: log
change: wizard-auth-and-mailbox-aliases
landed: 2026-07-25
---

# Wizard auth model and mailbox alias pre-fill

Reworked the discovered-account wizard into a two-step selection and taught it to pre-fill mailbox aliases. Moved the wizard capability forward: the discovery list now shows one entry per service, the authentication method is chosen in a second service-specific prompt (SASL mechanism for IMAP + SMTP, HTTP scheme for JMAP), and OAuth folds into the API-token credential prompt instead of being a dead-end row. The IMAP + SMTP flow tests each protocol as it configures it and asks whether SMTP shares the IMAP credentials. The account-name prompt is gone, the name being derived from the input.

The wizard pre-fills `mailbox.alias.*`: JMAP reads the RFC 8621 roles live over the tested connection, Gmail and Microsoft Graph map their fixed system-label ids and well-known folder names, and IMAP pins the reserved `INBOX`. The remaining IMAP special-use roles are tracked as a separate change (imap-special-use-aliases), blocked on upstream imap-codec.

Also touched the provider-quirks capability (SASL OAuth carries a login, the JMAP download host may differ, IMAP special-use is inbox-only) and the shared pimalaya-cli token picker (now combining keyrings and OAuth brokers behind an oauth gate).
